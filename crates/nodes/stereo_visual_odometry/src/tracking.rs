use color_eyre::{Result, eyre::Report};
use nalgebra::Isometry3;
use ros_z::time::Time;
use types::visual_odometry::{VisualOdometer, VisualOdometryDelta};

/// Outcome of processing one frame, including continuity failures.
#[derive(Debug)]
pub enum TrackingOutcome {
    Initializing,
    Estimated,
    Reset { error: Option<Report> },
}

#[derive(Debug)]
pub struct FrameOutput {
    pub outcome: TrackingOutcome,
    pub odometer: VisualOdometer,
}

impl FrameOutput {
    pub fn previous_to_current(&self) -> Option<Isometry3<f32>> {
        self.odometer
            .delta
            .as_ref()
            .map(|delta| delta.current_left_camera_to_previous_left_camera.inverse())
    }
}

/// Continuity policy, independent of inference, transport, and simulation.
#[derive(Default)]
pub(crate) struct TrackingState {
    previous_time: Option<Time>,
    epoch: u64,
    current_to_odometer: Isometry3<f32>,
}

impl TrackingState {
    pub(crate) fn finish(
        &mut self,
        time: Time,
        estimate: Result<Option<Isometry3<f32>>>,
    ) -> FrameOutput {
        let (outcome, delta) = match (self.previous_time, estimate) {
            (Some(previous_time), Ok(Some(previous_to_current))) => {
                let current_to_previous = previous_to_current.inverse();
                self.current_to_odometer *= current_to_previous;
                self.previous_time = Some(time);
                (
                    TrackingOutcome::Estimated,
                    Some(VisualOdometryDelta {
                        previous_time,
                        current_left_camera_to_previous_left_camera: current_to_previous,
                    }),
                )
            }
            (None, Ok(None)) => {
                self.previous_time = Some(time);
                (TrackingOutcome::Initializing, None)
            }
            (_, result) => {
                self.epoch = self.epoch.wrapping_add(1);
                self.previous_time = None;
                self.current_to_odometer = Isometry3::identity();
                (
                    TrackingOutcome::Reset {
                        error: result.err(),
                    },
                    None,
                )
            }
        };
        FrameOutput {
            outcome,
            odometer: VisualOdometer {
                time,
                epoch: self.epoch,
                delta,
                current_left_camera_to_visual_odometer: self.current_to_odometer,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuity_resets_on_missing_estimates_and_processing_errors() {
        let mut tracking = TrackingState::default();
        let time = Time::from_nanos;
        let step = Isometry3::translation(1.0, 2.0, 3.0);
        let initialized = tracking.finish(time(1), Ok(None));
        assert!(matches!(initialized.outcome, TrackingOutcome::Initializing));
        assert!(initialized.odometer.delta.is_none());
        for current in [2, 3] {
            let output = tracking.finish(time(current), Ok(Some(step)));
            assert!(matches!(output.outcome, TrackingOutcome::Estimated));
            assert_eq!(output.previous_to_current(), Some(step));
            let delta = output.odometer.delta.unwrap();
            assert_eq!(delta.previous_time, time(current - 1));
            assert_eq!(
                delta.current_left_camera_to_previous_left_camera,
                step.inverse()
            );
            assert_eq!(output.odometer.epoch, 0);
        }
        assert_eq!(
            tracking.current_to_odometer,
            step.inverse() * step.inverse()
        );
        let lost = tracking.finish(time(4), Ok(None));
        assert!(matches!(
            lost.outcome,
            TrackingOutcome::Reset { error: None }
        ));
        assert_eq!(lost.odometer.epoch, 1);
        assert!(lost.odometer.delta.is_none());
        assert_eq!(
            lost.odometer.current_left_camera_to_visual_odometer,
            Isometry3::identity()
        );
        assert!(matches!(
            tracking.finish(time(5), Ok(None)).outcome,
            TrackingOutcome::Initializing
        ));
        let resumed = tracking.finish(time(6), Ok(Some(step)));
        assert_eq!(resumed.odometer.delta.unwrap().previous_time, time(5));
        assert_eq!(
            resumed.odometer.current_left_camera_to_visual_odometer,
            step.inverse()
        );
        let failed = tracking.finish(time(7), Err(color_eyre::eyre::eyre!("inference failed")));
        assert!(matches!(
            failed.outcome,
            TrackingOutcome::Reset { error: Some(_) }
        ));
        assert_eq!(failed.odometer.epoch, 2);
        // An error while uninitialized also resets; the next valid frame initializes.
        let failed = tracking.finish(time(8), Err(color_eyre::eyre::eyre!("inference failed")));
        assert_eq!(failed.odometer.epoch, 3);
        assert!(matches!(
            tracking.finish(time(9), Ok(None)).outcome,
            TrackingOutcome::Initializing
        ));
    }
}
