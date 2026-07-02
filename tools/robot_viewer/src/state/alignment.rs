use std::{sync::Arc, time::Duration};

use ros_z::{cache::CacheInner, time::Time};
use types::time_wrapper::TimeWrapper;

use super::{CameraFrame, MAX_ALIGNED_STREAM_AGE};

pub(super) fn monotonic_anchor_time(
    preferred_anchor_time: Option<Time>,
    latest_camera_time: Option<Time>,
    display_anchor_time: Option<Time>,
    has_new_overlay_for_displayed_anchor: bool,
) -> Option<Time> {
    let Some(display_anchor_time) = display_anchor_time else {
        return preferred_anchor_time;
    };
    let Some(preferred_anchor_time) = preferred_anchor_time else {
        return None;
    };
    if preferred_anchor_time >= display_anchor_time {
        if preferred_anchor_time == display_anchor_time
            && let Some(latest_camera_time) = latest_camera_time
            && latest_camera_time > display_anchor_time
        {
            if has_new_overlay_for_displayed_anchor {
                return Some(preferred_anchor_time);
            }
            return Some(latest_camera_time);
        }
        return Some(preferred_anchor_time);
    }

    latest_camera_time
        .filter(|latest_camera_time| *latest_camera_time >= display_anchor_time)
        .or(Some(display_anchor_time))
}

pub(super) fn has_new_overlay_for_displayed_anchor(
    preferred_anchor_time: Option<Time>,
    association_anchor_time: Option<Time>,
    detection_anchor_time: Option<Time>,
    display_anchor_time: Option<Time>,
    displayed_anchor_has_field_mark_associations: bool,
    displayed_anchor_has_detected_objects: bool,
) -> bool {
    let Some(display_anchor_time) = display_anchor_time else {
        return false;
    };
    if preferred_anchor_time != Some(display_anchor_time) {
        return false;
    }

    let has_new_associations = association_anchor_time == Some(display_anchor_time)
        && !displayed_anchor_has_field_mark_associations;
    let has_new_detections = detection_anchor_time == Some(display_anchor_time)
        && !displayed_anchor_has_detected_objects;
    has_new_associations || has_new_detections
}

pub(super) fn latest_aligned_stream_time<T>(
    stream: &CacheInner<T>,
    camera_frames: &CacheInner<CameraFrame>,
    latest_camera_time: Option<Time>,
) -> Option<Time> {
    let latest_camera_time = latest_camera_time?;
    let mut stream_time = stream.latest_stamp_at_or_before(latest_camera_time)?;
    loop {
        if latest_camera_time.abs_diff(stream_time) > MAX_ALIGNED_STREAM_AGE {
            return None;
        }
        if camera_frames.get_exact(stream_time).is_some() {
            return Some(stream_time);
        }
        stream_time = stream.latest_stamp_before(stream_time)?;
    }
}

pub(super) fn exact_sample<T>(cache: &CacheInner<T>, time: Time) -> Option<TimeWrapper<Arc<T>>> {
    cache
        .get_exact(time)
        .map(|inner| TimeWrapper { time, inner })
}

pub(super) fn nearest_sample<T>(
    cache: &CacheInner<T>,
    time: Time,
    max_distance: Duration,
) -> Option<TimeWrapper<Arc<T>>> {
    let (sample_time, inner) = cache.get_nearest_with_stamp(time)?;
    (sample_time.abs_diff(time) <= max_distance).then(|| TimeWrapper {
        time: sample_time,
        inner,
    })
}

#[cfg(test)]
mod tests {
    use coordinate_systems::{Camera, Robot};
    use field_mark_association::FieldMarkAssociations;
    use kinematics::robot_kinematics::RobotKinematics;
    use linear_algebra::Isometry3;
    use projection::camera_matrix::CameraMatrix;
    use ros_z::time::Time;

    use super::*;
    use crate::state::{MAX_NEAREST_SAMPLE_DISTANCE, ViewerState};

    fn empty_associations() -> FieldMarkAssociations {
        FieldMarkAssociations {
            robot_to_camera: Isometry3::<Robot, Camera>::identity(),
            associations: Vec::new(),
        }
    }

    #[test]
    fn high_rate_alignment_streams_retain_delayed_render_samples() {
        let mut state = ViewerState::default();
        let anchor_time = Time::from_nanos(0);

        for index in 0..600 {
            let time = Time::from_nanos(index * 2_000_000);
            state.push_camera_matrix(time, CameraMatrix::default());
            state.push_robot_kinematics(time, RobotKinematics::default());
        }

        assert!(
            nearest_sample(
                &state.camera_matrices,
                anchor_time,
                MAX_NEAREST_SAMPLE_DISTANCE
            )
            .is_some()
        );
        assert!(
            nearest_sample(
                &state.robot_kinematics,
                anchor_time,
                MAX_NEAREST_SAMPLE_DISTANCE
            )
            .is_some()
        );
    }

    #[test]
    fn aligned_snapshot_prefers_associations_over_newer_detections() {
        let mut state = ViewerState::default();
        let association_time = Time::from_nanos(1_000_000_000);
        let detection_time = Time::from_nanos(1_100_000_000);

        state.push_camera_frame(association_time, CameraFrame::default());
        state.push_camera_frame(detection_time, CameraFrame::default());
        state.push_field_mark_associations(association_time, empty_associations());
        state.push_detected_objects(detection_time, Vec::new());

        let aligned = state.aligned_snapshot();

        assert_eq!(aligned.anchor_time, Some(association_time));
        assert!(aligned.field_mark_associations.is_some());
        assert!(aligned.detected_objects.is_none());
    }

    #[test]
    fn aligned_snapshot_uses_delayed_association_for_buffered_camera_frame() {
        let mut state = ViewerState::default();
        let association_time = Time::from_nanos(1_000_000_000);
        let latest_camera_time = Time::from_nanos(1_100_000_000);

        state.push_camera_frame(association_time, CameraFrame::default());
        state.push_camera_frame(latest_camera_time, CameraFrame::default());
        state.push_field_mark_associations(association_time, empty_associations());
        let aligned = state.aligned_snapshot();

        assert_eq!(aligned.anchor_time, Some(association_time));
        assert!(aligned.field_mark_associations.is_some());
    }

    #[test]
    fn aligned_snapshot_does_not_move_display_backwards_for_delayed_association() {
        let mut state = ViewerState::default();
        let old_time = Time::from_nanos(1_000_000_000);
        let new_time = Time::from_nanos(1_100_000_000);

        state.push_camera_frame(old_time, CameraFrame::default());
        state.push_camera_frame(new_time, CameraFrame::default());
        assert_eq!(state.aligned_snapshot().anchor_time, Some(new_time));

        state.push_field_mark_associations(old_time, empty_associations());
        let aligned = state.aligned_snapshot();

        assert_eq!(aligned.anchor_time, Some(new_time));
        assert!(aligned.field_mark_associations.is_none());
    }

    #[test]
    fn aligned_snapshot_accepts_newer_association_after_display_progresses() {
        let mut state = ViewerState::default();
        let old_time = Time::from_nanos(1_000_000_000);
        let new_time = Time::from_nanos(1_100_000_000);
        let newer_time = Time::from_nanos(1_200_000_000);

        state.push_camera_frame(old_time, CameraFrame::default());
        state.push_camera_frame(new_time, CameraFrame::default());
        assert_eq!(state.aligned_snapshot().anchor_time, Some(new_time));

        state.push_camera_frame(newer_time, CameraFrame::default());
        state.push_field_mark_associations(newer_time, empty_associations());
        let aligned = state.aligned_snapshot();

        assert_eq!(aligned.anchor_time, Some(newer_time));
        assert!(aligned.field_mark_associations.is_some());
    }

    #[test]
    fn aligned_snapshot_advances_past_displayed_association_when_newer_camera_exists() {
        let mut state = ViewerState::default();
        let association_time = Time::from_nanos(1_000_000_000);
        let latest_camera_time = Time::from_nanos(1_100_000_000);

        state.push_camera_frame(association_time, CameraFrame::default());
        state.push_field_mark_associations(association_time, empty_associations());
        assert_eq!(state.aligned_snapshot().anchor_time, Some(association_time));

        state.push_camera_frame(latest_camera_time, CameraFrame::default());
        let aligned = state.aligned_snapshot();

        assert_eq!(aligned.anchor_time, Some(latest_camera_time));
        assert!(aligned.field_mark_associations.is_none());
    }

    #[test]
    fn aligned_snapshot_keeps_current_frame_for_delayed_detection_before_advancing() {
        let mut state = ViewerState::default();
        let displayed_time = Time::from_nanos(1_000_000_000);
        let latest_camera_time = Time::from_nanos(1_100_000_000);

        state.push_camera_frame(displayed_time, CameraFrame::default());
        assert_eq!(state.aligned_snapshot().anchor_time, Some(displayed_time));

        state.push_detected_objects(displayed_time, Vec::new());
        state.push_camera_frame(latest_camera_time, CameraFrame::default());
        let aligned = state.aligned_snapshot();

        assert_eq!(aligned.anchor_time, Some(displayed_time));
        assert!(aligned.detected_objects.is_some());

        let aligned = state.aligned_snapshot();
        assert_eq!(aligned.anchor_time, Some(latest_camera_time));
        assert!(aligned.detected_objects.is_none());
    }

    #[test]
    fn aligned_snapshot_waits_for_next_detection_when_detector_is_live() {
        let mut state = ViewerState::default();
        let first_time = Time::from_nanos(1_000_000_000);
        let second_time = Time::from_nanos(1_100_000_000);

        state.objects_status.update_publishers(1);
        state.push_camera_frame(first_time, CameraFrame::default());
        state.push_detected_objects(first_time, Vec::new());
        assert_eq!(state.aligned_snapshot().anchor_time, Some(first_time));

        state.push_camera_frame(second_time, CameraFrame::default());
        let aligned = state.aligned_snapshot();

        assert_eq!(aligned.anchor_time, Some(first_time));
        assert!(aligned.detected_objects.is_some());

        state.push_detected_objects(second_time, Vec::new());
        let aligned = state.aligned_snapshot();

        assert_eq!(aligned.anchor_time, Some(second_time));
        assert!(aligned.detected_objects.is_some());
    }
}
