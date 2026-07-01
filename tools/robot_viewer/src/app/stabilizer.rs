use std::{sync::Arc, time::Duration};

use kinematics::robot_kinematics::RobotKinematics;
use projection::camera_matrix::CameraMatrix;
use ros_z::time::Time;
use types::time_wrapper::TimeWrapper;

use crate::state::AlignedViewerState;

const RENDER_SAMPLE_GRACE: Duration = Duration::from_millis(250);

#[derive(Default)]
pub(super) struct RenderSampleStabilizer {
    camera_matrix: Option<StabilizedSample<CameraMatrix>>,
    robot_kinematics: Option<StabilizedSample<RobotKinematics>>,
}

struct StabilizedSample<T> {
    anchor_time: Time,
    sample: TimeWrapper<Arc<T>>,
}

impl RenderSampleStabilizer {
    pub(super) fn stabilize(&mut self, state: &mut AlignedViewerState) {
        state.camera_matrix = stabilize_sample(
            &mut self.camera_matrix,
            state.camera_matrix.take(),
            state.anchor_time,
        );
        state.robot_kinematics = stabilize_sample(
            &mut self.robot_kinematics,
            state.robot_kinematics.take(),
            state.anchor_time,
        );
    }
}

fn stabilize_sample<T>(
    stabilized: &mut Option<StabilizedSample<T>>,
    sample: Option<TimeWrapper<Arc<T>>>,
    anchor_time: Option<Time>,
) -> Option<TimeWrapper<Arc<T>>> {
    match (sample, anchor_time) {
        (Some(sample), Some(anchor_time)) => {
            *stabilized = Some(StabilizedSample {
                anchor_time,
                sample: sample.clone(),
            });
            Some(sample)
        }
        (Some(sample), None) => {
            *stabilized = None;
            Some(sample)
        }
        (None, Some(anchor_time)) => {
            let Some(last) = stabilized.as_ref() else {
                return None;
            };
            if anchor_time.abs_diff(last.anchor_time) <= RENDER_SAMPLE_GRACE {
                Some(last.sample.clone())
            } else {
                *stabilized = None;
                None
            }
        }
        (None, None) => {
            *stabilized = None;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapped_sample(value: u8, time_nanos: i64) -> TimeWrapper<Arc<u8>> {
        TimeWrapper {
            time: Time::from_nanos(time_nanos),
            inner: Arc::new(value),
        }
    }

    #[test]
    fn stabilize_sample_reuses_last_sample_during_short_gap() {
        let mut stabilized = None;
        let first_anchor = Time::from_nanos(1_000_000_000);
        let first_sample = wrapped_sample(7, first_anchor.as_nanos());

        let sample = stabilize_sample(&mut stabilized, Some(first_sample), Some(first_anchor))
            .expect("current sample should be used");
        assert_eq!(*sample.inner, 7);

        let gap_anchor = first_anchor.saturating_add(RENDER_SAMPLE_GRACE);
        let sample = stabilize_sample::<u8>(&mut stabilized, None, Some(gap_anchor))
            .expect("last sample should be reused inside grace window");
        assert_eq!(*sample.inner, 7);
    }

    #[test]
    fn stabilize_sample_expires_after_grace_window() {
        let mut stabilized = None;
        let first_anchor = Time::from_nanos(1_000_000_000);
        let first_sample = wrapped_sample(7, first_anchor.as_nanos());

        stabilize_sample(&mut stabilized, Some(first_sample), Some(first_anchor))
            .expect("current sample should be used");

        let expired_anchor = first_anchor
            .saturating_add(RENDER_SAMPLE_GRACE)
            .saturating_add(Duration::from_nanos(1));
        assert!(stabilize_sample::<u8>(&mut stabilized, None, Some(expired_anchor)).is_none());
        assert!(stabilized.is_none());
    }
}
