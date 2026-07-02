use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use coordinate_systems::{Field, Robot};
use field_mark_association::FieldMarkAssociations;
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Isometry3;
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::{cache::CacheInner, time::Time};
use types::{
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

mod alignment;

use alignment::{
    exact_sample, has_new_overlay_for_displayed_anchor, latest_aligned_stream_time,
    monotonic_anchor_time, nearest_sample,
};

pub(crate) type SharedState = Arc<Mutex<ViewerState>>;

const CAMERA_FRAME_BUFFER_CAPACITY: usize = 12;
const STREAM_BUFFER_CAPACITY: usize = 64;
const HIGH_RATE_STREAM_BUFFER_CAPACITY: usize = 1024;
const MAX_ALIGNED_STREAM_AGE: Duration = Duration::from_secs(1);
const MAX_NEAREST_SAMPLE_DISTANCE: Duration = Duration::from_millis(100);

pub(crate) struct ViewerState {
    pub(crate) connection: ConnectionStatus,
    pub(crate) field_dimensions: Option<FieldDimensions>,
    pub(crate) localization: Option<Isometry3<Field, Robot>>,
    pub(crate) visual_odometer: Option<nalgebra::Isometry3<f32>>,
    pub(crate) robot_kinematics: CacheInner<RobotKinematics>,
    pub(crate) camera_matrices: CacheInner<CameraMatrix>,
    pub(crate) calibrated_intrinsics: Option<Intrinsic>,
    pub(crate) camera_frames: CacheInner<CameraFrame>,
    pub(crate) camera_sequence: u64,
    display_anchor_time: Option<Time>,
    displayed_anchor_has_detected_objects: bool,
    displayed_anchor_has_field_mark_associations: bool,
    pub(crate) detected_objects: CacheInner<Vec<Object<RobocupObjectLabel>>>,
    pub(crate) field_mark_associations: CacheInner<FieldMarkAssociations>,
    pub(crate) field_status: StreamStatus,
    pub(crate) localization_status: StreamStatus,
    pub(crate) visual_odometer_status: StreamStatus,
    pub(crate) robot_kinematics_status: StreamStatus,
    pub(crate) camera_matrix_status: StreamStatus,
    pub(crate) calibrated_intrinsics_status: StreamStatus,
    pub(crate) camera_status: StreamStatus,
    pub(crate) objects_status: StreamStatus,
    pub(crate) field_mark_associations_status: StreamStatus,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            connection: ConnectionStatus::default(),
            field_dimensions: None,
            localization: None,
            visual_odometer: None,
            robot_kinematics: CacheInner::new(HIGH_RATE_STREAM_BUFFER_CAPACITY),
            camera_matrices: CacheInner::new(HIGH_RATE_STREAM_BUFFER_CAPACITY),
            calibrated_intrinsics: None,
            camera_frames: CacheInner::new(CAMERA_FRAME_BUFFER_CAPACITY),
            camera_sequence: 0,
            display_anchor_time: None,
            displayed_anchor_has_detected_objects: false,
            displayed_anchor_has_field_mark_associations: false,
            detected_objects: CacheInner::new(STREAM_BUFFER_CAPACITY),
            field_mark_associations: CacheInner::new(STREAM_BUFFER_CAPACITY),
            field_status: StreamStatus::default(),
            localization_status: StreamStatus::default(),
            visual_odometer_status: StreamStatus::default(),
            robot_kinematics_status: StreamStatus::default(),
            camera_matrix_status: StreamStatus::default(),
            calibrated_intrinsics_status: StreamStatus::default(),
            camera_status: StreamStatus::default(),
            objects_status: StreamStatus::default(),
            field_mark_associations_status: StreamStatus::default(),
        }
    }
}

impl ViewerState {
    pub(crate) fn push_camera_frame(&mut self, time: Time, frame: CameraFrame) {
        self.camera_frames.insert(time, frame);
    }

    pub(crate) fn push_robot_kinematics(&mut self, time: Time, value: RobotKinematics) {
        self.robot_kinematics.insert(time, value);
    }

    pub(crate) fn push_camera_matrix(&mut self, time: Time, value: CameraMatrix) {
        self.camera_matrices.insert(time, value);
    }

    pub(crate) fn push_detected_objects(
        &mut self,
        time: Time,
        value: Vec<Object<RobocupObjectLabel>>,
    ) {
        self.detected_objects.insert(time, value);
    }

    pub(crate) fn push_field_mark_associations(
        &mut self,
        time: Time,
        value: FieldMarkAssociations,
    ) {
        self.field_mark_associations.insert(time, value);
    }

    pub(crate) fn status_snapshot(&self) -> ViewerStatusSnapshot {
        ViewerStatusSnapshot {
            connection: self.connection.clone(),
            field_status: self.field_status.clone(),
            localization_status: self.localization_status.clone(),
            visual_odometer_status: self.visual_odometer_status.clone(),
            robot_kinematics_status: self.robot_kinematics_status.clone(),
            camera_matrix_status: self.camera_matrix_status.clone(),
            calibrated_intrinsics_status: self.calibrated_intrinsics_status.clone(),
            camera_status: self.camera_status.clone(),
            objects_status: self.objects_status.clone(),
            field_mark_associations_status: self.field_mark_associations_status.clone(),
        }
    }

    /// Builds the data snapshot rendered by the camera panel and 3D scene for a selected frame.
    ///
    /// The anchor is the newest camera frame that has aligned field-mark associations, then the
    /// newest camera frame that has aligned object detections, and finally the newest camera frame.
    /// Object detections and field-mark associations are exact timestamp matches. Camera matrix and
    /// kinematics are nearest samples within `MAX_NEAREST_SAMPLE_DISTANCE`; the UI may briefly reuse
    /// the last valid render sample to avoid flicker from message-ordering jitter. Pose sources and
    /// calibrated intrinsics remain latest-value streams because their current topics do not carry a
    /// frame timestamp; the UI labels pose sources as latest for this reason.
    pub(crate) fn aligned_snapshot(&mut self) -> AlignedViewerState {
        let latest_camera_time = self.camera_frames.latest_stamp();
        let association_anchor_time = latest_aligned_stream_time(
            &self.field_mark_associations,
            &self.camera_frames,
            latest_camera_time,
        );
        let detection_anchor_time = latest_aligned_stream_time(
            &self.detected_objects,
            &self.camera_frames,
            latest_camera_time,
        );
        let wait_for_detected_objects =
            self.objects_status.publisher_count > 0 && !self.detected_objects.is_empty();
        let preferred_anchor_time =
            association_anchor_time
                .or(detection_anchor_time)
                .or_else(|| {
                    if wait_for_detected_objects && self.display_anchor_time.is_some() {
                        self.display_anchor_time
                    } else {
                        latest_camera_time
                    }
                });
        let latest_camera_time_for_monotonic =
            if wait_for_detected_objects && detection_anchor_time <= self.display_anchor_time {
                self.display_anchor_time
            } else {
                latest_camera_time
            };
        let anchor_time = monotonic_anchor_time(
            preferred_anchor_time,
            latest_camera_time_for_monotonic,
            self.display_anchor_time,
            has_new_overlay_for_displayed_anchor(
                preferred_anchor_time,
                association_anchor_time,
                detection_anchor_time,
                self.display_anchor_time,
                self.displayed_anchor_has_field_mark_associations,
                self.displayed_anchor_has_detected_objects,
            ),
        );

        let camera_frame = anchor_time.and_then(|time| exact_sample(&self.camera_frames, time));
        let camera_matrix = anchor_time.and_then(|time| {
            nearest_sample(&self.camera_matrices, time, MAX_NEAREST_SAMPLE_DISTANCE)
        });
        let robot_kinematics = anchor_time.and_then(|time| {
            nearest_sample(&self.robot_kinematics, time, MAX_NEAREST_SAMPLE_DISTANCE)
        });
        let detected_objects =
            anchor_time.and_then(|time| exact_sample(&self.detected_objects, time));
        let field_mark_associations =
            anchor_time.and_then(|time| exact_sample(&self.field_mark_associations, time));

        if anchor_time != self.display_anchor_time {
            self.display_anchor_time = anchor_time;
            self.displayed_anchor_has_detected_objects = detected_objects.is_some();
            self.displayed_anchor_has_field_mark_associations = field_mark_associations.is_some();
        } else {
            self.displayed_anchor_has_detected_objects |= detected_objects.is_some();
            self.displayed_anchor_has_field_mark_associations |= field_mark_associations.is_some();
        }

        AlignedViewerState {
            anchor_time,
            field_dimensions: self.field_dimensions,
            latest_localization: self.localization,
            latest_visual_odometer: self.visual_odometer,
            robot_kinematics,
            camera_matrix,
            latest_calibrated_intrinsics: self.calibrated_intrinsics,
            camera_frame,
            detected_objects,
            field_mark_associations,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct ViewerStatusSnapshot {
    pub(crate) connection: ConnectionStatus,
    pub(crate) field_status: StreamStatus,
    pub(crate) localization_status: StreamStatus,
    pub(crate) visual_odometer_status: StreamStatus,
    pub(crate) robot_kinematics_status: StreamStatus,
    pub(crate) camera_matrix_status: StreamStatus,
    pub(crate) calibrated_intrinsics_status: StreamStatus,
    pub(crate) camera_status: StreamStatus,
    pub(crate) objects_status: StreamStatus,
    pub(crate) field_mark_associations_status: StreamStatus,
}

/// Timestamp-aligned render input for one displayed camera frame.
///
/// Optional exact-match streams stay `None` when the corresponding timestamp has not arrived, so
/// the UI can show “unavailable” instead of pretending the stream produced an empty result.
#[derive(Clone, Default)]
pub(crate) struct AlignedViewerState {
    pub(crate) anchor_time: Option<Time>,
    pub(crate) field_dimensions: Option<FieldDimensions>,
    pub(crate) latest_localization: Option<Isometry3<Field, Robot>>,
    pub(crate) latest_visual_odometer: Option<nalgebra::Isometry3<f32>>,
    pub(crate) robot_kinematics: Option<TimeWrapper<Arc<RobotKinematics>>>,
    pub(crate) camera_matrix: Option<TimeWrapper<Arc<CameraMatrix>>>,
    pub(crate) latest_calibrated_intrinsics: Option<Intrinsic>,
    pub(crate) camera_frame: Option<TimeWrapper<Arc<CameraFrame>>>,
    pub(crate) detected_objects: Option<TimeWrapper<Arc<Vec<Object<RobocupObjectLabel>>>>>,
    pub(crate) field_mark_associations: Option<TimeWrapper<Arc<FieldMarkAssociations>>>,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(crate) enum PoseSource {
    #[default]
    Localization,
    VisualOdometer,
}

#[derive(Clone, Default)]
pub(crate) struct CameraFrame {
    pub(crate) sequence: u64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

#[derive(Clone, Default)]
pub(crate) enum ConnectionStatus {
    #[default]
    Starting,
    Connecting,
    Subscribed,
    Error(String),
}

#[derive(Clone, Default)]
pub(crate) struct StreamStatus {
    pub(crate) state: StreamState,
    pub(crate) publisher_count: usize,
    pub(crate) detail: Option<String>,
}

impl StreamStatus {
    pub(crate) fn update_publishers(&mut self, publisher_count: usize) {
        self.publisher_count = publisher_count;
        if matches!(self.state, StreamState::Waiting | StreamState::Matched) {
            self.state = if publisher_count > 0 {
                StreamState::Matched
            } else {
                StreamState::Waiting
            };
        }
    }

    pub(crate) fn mark_live(&mut self, publisher_count: usize) {
        self.state = StreamState::Live;
        self.publisher_count = publisher_count;
        self.detail = None;
    }

    pub(crate) fn mark_value(&mut self, publisher_count: usize, has_value: bool) {
        self.state = if has_value {
            StreamState::Live
        } else {
            StreamState::Empty
        };
        self.publisher_count = publisher_count;
        self.detail = None;
    }

    pub(crate) fn mark_error(&mut self, publisher_count: usize, detail: String) {
        self.state = StreamState::Error;
        self.publisher_count = publisher_count;
        self.detail = Some(detail);
    }
}

#[derive(Clone, Default)]
pub(crate) enum StreamState {
    #[default]
    Waiting,
    Matched,
    Live,
    Empty,
    Error,
}
