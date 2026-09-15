//! Alternating ground targets; gaze geometry and joint trajectories are evaluated by the caller.

use std::time::Duration;

use color_eyre::{
    Result,
    eyre::{ensure, eyre},
};
use coordinate_systems::Ground;
use kinematics::joints::head::HeadJoints;
use linear_algebra::{Point2, Rotation2};
use ros_z::time::Time;
use types::{motion_command::ImageRegion, support_foot::Side};

use crate::{joint_control::MotionProgress, parameters::Parameters};

pub struct GlanceTarget {
    pub position: Point2<Ground>,
    pub image_region: ImageRegion,
    pub travel_speed: HeadJoints<f32>,
}

#[derive(Debug)]
pub struct GlanceTimeout {
    pub side: Side,
    pub target: Point2<Ground>,
    pub progress: MotionProgress,
    pub elapsed: Duration,
}

struct PendingGlance {
    target: Point2<Ground>,
    elapsed: Duration,
    maximum_duration: Duration,
}

struct ActiveGlance {
    side: Side,
    elapsed: Duration,
    last_update: Time,
    tracking: bool,
    pending: Option<PendingGlance>,
}

impl ActiveGlance {
    fn new(now: Time) -> Self {
        Self {
            side: Side::Left,
            elapsed: Duration::ZERO,
            last_update: now,
            tracking: false,
            pending: None,
        }
    }
}

#[derive(Default)]
pub struct GlanceState {
    active: Option<ActiveGlance>,
}

impl GlanceState {
    /// Select the current side's target, preserving distance from the Ground origin.
    /// The coordinator retains target height and solves gaze geometry for this point.
    /// Target updates preserve phase; gaps and clock rollback restart on the left.
    /// After joint control evaluates this target, call `advance` with that output's
    /// progress. Never reuse progress from an older target or from a hold command.
    pub fn update(
        &mut self,
        target: Point2<Ground>,
        parameters: &Parameters,
        now: Time,
    ) -> Result<GlanceTarget> {
        parameters
            .glance
            .validate()
            .map_err(|error| eyre!("glance.{error}"))?;
        ensure!(
            target.inner.coords.iter().all(|value| value.is_finite()),
            "glance target must be finite"
        );
        let active = self.active.get_or_insert_with(|| ActiveGlance::new(now));
        if now < active.last_update
            || now.duration_since(active.last_update) > parameters.joint_control.reseed_after
        {
            *active = ActiveGlance::new(now);
        }
        let angle = match active.side {
            Side::Left => parameters.glance.angle,
            Side::Right => -parameters.glance.angle,
        };
        let position = Rotation2::<Ground, Ground>::new(angle) * target;
        ensure!(
            position.inner.coords.iter().all(|value| value.is_finite()),
            "glance offset target must be finite"
        );
        active.pending = Some(PendingGlance {
            target: position,
            elapsed: if active.tracking {
                now.duration_since(active.last_update)
            } else {
                Duration::ZERO
            },
            maximum_duration: parameters.glance.maximum_phase_duration,
        });
        active.last_update = now;
        Ok(GlanceTarget {
            position,
            image_region: ImageRegion::Center,
            travel_speed: parameters.glance.travel_speed,
        })
    }

    /// Consume feedback for the just-selected target, switching for the next request.
    /// Position arrival suffices even while moving; there is no dwell. Pass None on
    /// geometry/planning failure to preserve the side and pause its deadline while
    /// the coordinator holds. Each selected target consumes feedback at most once.
    pub fn advance(&mut self, progress: Option<&MotionProgress>) -> Option<GlanceTimeout> {
        let active = self.active.as_mut()?;
        let pending = active.pending.take();
        let Some(progress) = progress else {
            active.tracking = false;
            return None;
        };
        let pending = pending?;
        active.elapsed += pending.elapsed;
        active.tracking = true;
        let expired = active.elapsed >= pending.maximum_duration;
        let timeout = (expired && !progress.position_reached).then_some(GlanceTimeout {
            side: active.side,
            target: pending.target,
            progress: *progress,
            elapsed: active.elapsed,
        });
        if progress.position_reached || expired {
            active.side = active.side.opposite();
            active.elapsed = Duration::ZERO;
            active.tracking = false;
        }
        timeout
    }

    /// Call when leaving LookLeftAndRightOf, including damping or another pattern.
    pub fn reset(&mut self) {
        self.active = None;
    }
}

#[cfg(test)]
mod tests {
    use json5::from_str;
    use kinematics::{
        forward::{head_to_left_camera, head_to_robot},
        joints::head::HeadJoint,
    };
    use linear_algebra::{Isometry3, nalgebra, point, vector};
    use projection::camera_matrix::CameraMatrix;
    use serde::Deserialize;
    use types::joint_limits::JointLimits;

    use super::*;
    use crate::{
        joint_control::{
            ConstraintCause, HeadObservation, JointController, JointTarget, KinematicState,
        },
        look_at::{GazeGeometry, look_at},
    };

    fn parameters() -> Parameters {
        let parameters: Parameters = from_str(include_str!(
            "../../../../../etc/parameters/base/head_motion.json5"
        ))
        .unwrap();
        parameters.validate().unwrap();
        parameters
    }

    fn at(millis: u64) -> Time {
        Time::zero() + Duration::from_millis(millis)
    }

    fn progress(position_reached: bool) -> MotionProgress {
        MotionProgress {
            requested_target: HeadJoints::fill(0.0),
            effective_target: HeadJoints::fill(0.0),
            position_reached,
            target_reached: false,
            constrained: false,
        }
    }

    #[test]
    fn offsets_preserve_range_and_signed_bearing_for_targets_in_every_direction() {
        let parameters = parameters();
        for target in [
            point![2.0, 0.0],
            point![0.0, 2.0],
            point![-2.0, 0.0],
            point![0.0, -2.0],
            point![1.3, -0.8],
        ] {
            let mut state = GlanceState::default();
            for (millis, sign) in [(0, 1.0), (10, -1.0)] {
                let output = state.update(target, &parameters, at(millis)).unwrap();
                let offset = output.position;
                let range_squared = target.inner.coords.norm_squared();
                let cosine = (target.x() * offset.x() + target.y() * offset.y()) / range_squared;
                let sine = (target.x() * offset.y() - target.y() * offset.x()) / range_squared;
                assert!((offset.inner.coords.norm_squared() - range_squared).abs() < 1e-5);
                assert!((cosine - parameters.glance.angle.cos()).abs() < 1e-6);
                assert!((sine - sign * parameters.glance.angle.sin()).abs() < 1e-6);
                assert_eq!(output.image_region, ImageRegion::Center);
                assert_eq!(output.travel_speed, parameters.glance.travel_speed);
                assert!(state.advance(Some(&progress(true))).is_none());
            }
        }
    }

    #[test]
    fn moving_targets_reverse_on_measured_position_without_dwell_and_keep_derivative_continuity() {
        #[derive(Deserialize)]
        struct Global {
            joint_limits: JointLimits,
        }
        let global: Global = from_str(include_str!(
            "../../../../../etc/parameters/base/global.json5"
        ))
        .unwrap();
        let parameters = parameters();
        let camera = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.55, 0.65],
            nalgebra::point![0.5, 0.5],
            vector![640.0, 480.0],
            Isometry3::identity(),
            head_to_robot(&HeadJoints::default()).inverse(),
            head_to_left_camera(-0.2),
        );
        let geometry = GazeGeometry {
            camera_matrix: &camera,
            ground_to_robot: Isometry3::from_translation(0.0, 0.0, -0.55),
        };
        let mut glance = GlanceState::default();
        let mut controller = JointController::default();
        let mut observation = HeadObservation {
            positions: HeadJoints::fill(0.0),
            velocities: HeadJoints::fill(0.0),
        };
        let mut previous: Option<HeadJoints<KinematicState>> = None;
        let mut reversals_while_moving = 0;
        for sample in 0..1000 {
            let now = at(sample * 10);
            let ball = point![2.0, 0.1 * (sample as f32 * 0.003).sin()];
            let target = glance.update(ball, &parameters, now).unwrap();
            let side = glance.active.as_ref().unwrap().side;
            // Target height belongs to the coordinator and is retained through offsetting.
            let angles = look_at(
                target.position,
                0.105,
                target.image_region,
                &geometry,
                &parameters.image_region_parameters,
                observation.positions,
            )
            .unwrap();
            let output = controller
                .update(
                    JointTarget::MoveTo {
                        position: angles,
                        travel_speed: target.travel_speed,
                    },
                    &observation,
                    &parameters.joint_control,
                    &global.joint_limits,
                    now,
                )
                .unwrap();
            assert!(
                !output
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.cause == ConstraintCause::PositionRecovery)
            );
            let progress = output.progress.as_ref().unwrap();
            assert!(
                glance.advance(Some(progress)).is_none(),
                "normal glancing must not need its fallback deadline"
            );
            let next_side = glance.active.as_ref().unwrap().side;
            assert_eq!(
                next_side != side,
                progress.position_reached,
                "reverse on position arrival without dwelling"
            );
            if next_side != side && !progress.target_reached {
                reversals_while_moving += 1;
            }
            if let Some(previous) = previous {
                for joint in [HeadJoint::Yaw, HeadJoint::Pitch] {
                    assert!(
                        (output.reference[joint].velocity - previous[joint].velocity).abs()
                            <= f64::from(parameters.joint_control.maximum_acceleration[joint])
                                * 0.01
                                + 1e-6
                    );
                    assert!(
                        (output.reference[joint].acceleration - previous[joint].acceleration).abs()
                            <= f64::from(parameters.joint_control.maximum_jerk[joint]) * 0.01
                                + 1e-6
                    );
                }
            }
            previous = Some(output.reference);
            observation.positions = HeadJoints {
                yaw: output.commands.yaw.position,
                pitch: output.commands.pitch.position,
            };
            observation.velocities = HeadJoints {
                yaw: output.commands.yaw.velocity,
                pitch: output.commands.pitch.velocity,
            };
        }
        assert!(
            reversals_while_moving >= 4,
            "must not wait for the moving head to settle"
        );
    }

    #[test]
    fn moving_targets_keep_the_deadline_holds_pause_it_and_reactivation_restarts_left() {
        let mut parameters = parameters();
        parameters.joint_control.reseed_after = Duration::from_secs(1);
        parameters.glance.maximum_phase_duration = Duration::from_millis(300);
        let mut state = GlanceState::default();
        let feedback = progress(false);
        for millis in [0, 100, 200, 300, 400, 500, 600] {
            let target = point![2.0, millis as f32 * 0.001];
            state.update(target, &parameters, at(millis)).unwrap();
            let timeout = state.advance(if (200..=300).contains(&millis) {
                None
            } else {
                Some(&feedback)
            });
            if millis == 600 {
                let timeout = timeout.unwrap();
                assert_eq!(timeout.side, Side::Left);
                assert_eq!(timeout.elapsed, Duration::from_millis(300));
                assert_eq!(state.active.as_ref().unwrap().side, Side::Right);
                assert!(
                    state.advance(Some(&feedback)).is_none(),
                    "feedback must be consumed only once"
                );
            } else {
                assert!(timeout.is_none());
                assert_eq!(state.active.as_ref().unwrap().side, Side::Left);
            }
        }
        state
            .update(point![2.0, 0.0], &parameters, at(1800))
            .unwrap();
        assert_eq!(state.active.as_ref().unwrap().side, Side::Left);
        state.advance(Some(&progress(true)));
        state
            .update(point![2.0, 0.0], &parameters, at(1700))
            .unwrap();
        assert_eq!(state.active.as_ref().unwrap().side, Side::Left);
        state.advance(Some(&progress(true)));
        state.reset();
        state
            .update(point![2.0, 0.0], &parameters, at(1710))
            .unwrap();
        assert_eq!(state.active.as_ref().unwrap().side, Side::Left);
        state.advance(Some(&feedback));
        assert!(
            state
                .update(point![f32::NAN, 0.0], &parameters, at(1720))
                .is_err()
        );
        state.advance(None);
        assert_eq!(state.active.as_ref().unwrap().side, Side::Left);
        state
            .update(point![2.0, 0.0], &parameters, at(1730))
            .unwrap();
        state.advance(Some(&feedback));
        assert_eq!(
            state.active.as_ref().unwrap().elapsed,
            Duration::ZERO,
            "an invalid target must pause timing even though update returned an error"
        );
    }
}
