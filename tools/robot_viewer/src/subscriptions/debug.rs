use std::{
    collections::{BTreeSet, VecDeque},
    num::NonZeroUsize,
    sync::Arc,
};

use color_eyre::Result;
use coordinate_systems::{Field, Robot};
use field_mark_association::FieldMarkAssociations;
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Isometry3;
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::{pubsub::PublicationId, time::Time};
use ros_z_debug::{
    CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot, RetentionPolicy, TopicObservation,
    TopicObservationStatus, TopicObserver,
};
use types::{
    field_dimensions::FieldDimensions, time_wrapper::TimeWrapper, visual_odometry::VisualOdometer,
};

use crate::state::{StreamStatus, ViewerState};

use super::topics::{
    CALIBRATED_INTRINSICS_TOPIC, CAMERA_MATRIX_TOPIC, DEBUG_HIGH_RATE_HISTORY_CAPACITY,
    DEBUG_HISTORY_WINDOW, DEBUG_STREAM_HISTORY_CAPACITY, FIELD_DIMENSIONS_TOPIC,
    FIELD_MARK_ASSOCIATIONS_TOPIC, LOCALIZATION_TOPIC, PROCESSED_PUBLICATION_CAPACITY,
    ROBOT_KINEMATICS_TOPIC, VISUAL_ODOMETER_TOPIC,
};

pub(super) struct DebugSubscriptions {
    field_dimensions: TopicObservation<FieldDimensions>,
    localization: TopicObservation<Option<Isometry3<Field, Robot>>>,
    visual_odometer: TopicObservation<VisualOdometer>,
    robot_kinematics: WindowedDebugStream<RobotKinematics>,
    camera_matrix: WindowedDebugStream<CameraMatrix>,
    calibrated_intrinsics: TopicObservation<Intrinsic>,
    field_mark_associations: WindowedDebugStream<FieldMarkAssociations>,
}

impl DebugSubscriptions {
    pub(super) fn build(observer: &TopicObserver) -> Result<Self> {
        let high_rate_history = RetentionPolicy::time_window_with_max_samples(
            DEBUG_HISTORY_WINDOW,
            NonZeroUsize::new(DEBUG_HIGH_RATE_HISTORY_CAPACITY).expect("capacity is non-zero"),
        )?;
        let stream_history = RetentionPolicy::time_window_with_max_samples(
            DEBUG_HISTORY_WINDOW,
            NonZeroUsize::new(DEBUG_STREAM_HISTORY_CAPACITY).expect("capacity is non-zero"),
        )?;

        Ok(Self {
            field_dimensions: observer
                .observe_typed::<FieldDimensions>(FIELD_DIMENSIONS_TOPIC)?
                .spawn(),
            localization: observer
                .observe_typed::<Option<Isometry3<Field, Robot>>>(LOCALIZATION_TOPIC)?
                .spawn(),
            visual_odometer: observer
                .observe_typed::<VisualOdometer>(VISUAL_ODOMETER_TOPIC)?
                .spawn(),
            robot_kinematics: WindowedDebugStream::new(
                observer
                    .observe_typed::<TimeWrapper<RobotKinematics>>(ROBOT_KINEMATICS_TOPIC)?
                    .retention(high_rate_history)
                    .spawn(),
            ),
            camera_matrix: WindowedDebugStream::new(
                observer
                    .observe_typed::<TimeWrapper<CameraMatrix>>(CAMERA_MATRIX_TOPIC)?
                    .retention(high_rate_history)
                    .spawn(),
            ),
            calibrated_intrinsics: observer
                .observe_typed::<Intrinsic>(CALIBRATED_INTRINSICS_TOPIC)?
                .spawn(),
            field_mark_associations: WindowedDebugStream::new(
                observer
                    .observe_typed::<TimeWrapper<FieldMarkAssociations>>(
                        FIELD_MARK_ASSOCIATIONS_TOPIC,
                    )?
                    .retention(stream_history)
                    .spawn(),
            ),
        })
    }

    pub(super) fn refresh(&mut self, state: &mut ViewerState) {
        if let Some(record) = self.field_dimensions.latest() {
            state.field_dimensions = Some(record.value.clone());
        }
        update_debug_status(
            &mut state.field_status,
            &self.field_dimensions,
            state.field_dimensions.is_some(),
        );

        if let Some(record) = self.localization.latest() {
            state.localization = record.value.clone();
        }
        update_debug_status(
            &mut state.localization_status,
            &self.localization,
            state.localization.is_some(),
        );

        if let Some(record) = self.visual_odometer.latest() {
            state.visual_odometer = Some(record.value.current_left_camera_to_visual_odometer);
        }
        update_debug_status(
            &mut state.visual_odometer_status,
            &self.visual_odometer,
            state.visual_odometer.is_some(),
        );

        for record in self.robot_kinematics.drain_new() {
            state.push_robot_kinematics(record.value.time, record.value.inner.clone());
        }
        update_debug_status(
            &mut state.robot_kinematics_status,
            self.robot_kinematics.observation(),
            self.robot_kinematics.latest().is_some(),
        );

        for record in self.camera_matrix.drain_new() {
            state.push_camera_matrix(record.value.time, record.value.inner.clone());
        }
        update_debug_status(
            &mut state.camera_matrix_status,
            self.camera_matrix.observation(),
            self.camera_matrix.latest().is_some(),
        );

        if let Some(record) = self.calibrated_intrinsics.latest() {
            state.calibrated_intrinsics = Some(record.value.clone());
        }
        update_debug_status(
            &mut state.calibrated_intrinsics_status,
            &self.calibrated_intrinsics,
            state.calibrated_intrinsics.is_some(),
        );

        for record in self.field_mark_associations.drain_new() {
            state.push_field_mark_associations(record.value.time, record.value.inner.clone());
        }
        update_debug_status(
            &mut state.field_mark_associations_status,
            self.field_mark_associations.observation(),
            self.field_mark_associations.latest().is_some(),
        );
    }
}

struct WindowedDebugStream<T> {
    observation: TopicObservation<TimeWrapper<T>>,
    processed: ProcessedPublications,
}

impl<T> WindowedDebugStream<T> {
    fn new(observation: TopicObservation<TimeWrapper<T>>) -> Self {
        Self {
            observation,
            processed: ProcessedPublications::default(),
        }
    }

    fn observation(&self) -> &TopicObservation<TimeWrapper<T>> {
        &self.observation
    }

    fn latest(&self) -> Option<Arc<ros_z_debug::SampleRecord<TimeWrapper<T>>>> {
        self.observation.latest()
    }

    fn drain_new(&mut self) -> Vec<Arc<ros_z_debug::SampleRecord<TimeWrapper<T>>>> {
        self.observation
            .window(Time::zero(), Time::from_nanos(i64::MAX))
            .into_iter()
            .filter(|record| self.processed.accept(record.publication_id))
            .collect()
    }
}

struct ProcessedPublications {
    seen: BTreeSet<PublicationId>,
    order: VecDeque<PublicationId>,
}

impl Default for ProcessedPublications {
    fn default() -> Self {
        Self {
            seen: BTreeSet::new(),
            order: VecDeque::with_capacity(PROCESSED_PUBLICATION_CAPACITY),
        }
    }
}

impl ProcessedPublications {
    fn accept(&mut self, publication_id: PublicationId) -> bool {
        if !self.seen.insert(publication_id) {
            return false;
        }

        self.order.push_back(publication_id);
        while self.order.len() > PROCESSED_PUBLICATION_CAPACITY {
            if let Some(oldest) = self.order.pop_front() {
                self.seen.remove(&oldest);
            }
        }

        true
    }
}

fn update_debug_status<T>(
    status: &mut StreamStatus,
    observation: &TopicObservation<T>,
    has_value: bool,
) {
    match observation.status() {
        TopicObservationStatus::Building | TopicObservationStatus::Rebuilding { .. } => {
            status.update_publishers(0)
        }
        TopicObservationStatus::Observing { cache } => {
            update_cached_status(status, &cache, has_value)
        }
        TopicObservationStatus::Retrying {
            previous_cache,
            error,
        } => {
            if let Some(cache) = previous_cache {
                update_cached_status(status, &cache, has_value);
            } else {
                status.mark_error(0, error);
            }
        }
        TopicObservationStatus::Blocked {
            previous_cache,
            reason,
        } => {
            if let Some(cache) = previous_cache {
                update_cached_status(status, &cache, has_value);
            } else {
                status.mark_error(0, format!("observation blocked: {reason:?}"));
            }
        }
        TopicObservationStatus::Closed => status.mark_error(0, "observation closed".to_string()),
        _ => status.update_publishers(0),
    }
}

fn update_cached_status(
    status: &mut StreamStatus,
    cache: &CachedSubscriptionStatusSnapshot,
    has_value: bool,
) {
    match cache.status() {
        CachedSubscriptionStatus::WaitingForFirstSample => status.update_publishers(1),
        CachedSubscriptionStatus::Ready => status.mark_value(1, has_value),
        CachedSubscriptionStatus::ProtocolError { .. }
        | CachedSubscriptionStatus::DecodeError { .. } => {
            status.mark_error(
                1,
                cache.message().unwrap_or("subscription error").to_string(),
            );
        }
        CachedSubscriptionStatus::Closed => {
            status.mark_error(0, "subscription closed".to_string());
        }
        _ => status.update_publishers(0),
    }
}
