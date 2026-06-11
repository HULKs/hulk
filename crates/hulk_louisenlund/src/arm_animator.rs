use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::{
    Result,
    eyre::{Context as _, ensure, eyre},
};
use log::warn;
use serde::{Deserialize, Serialize};
use tokio::time::MissedTickBehavior;

use booster::{CommandType, JointsMotorState, LowCommand, MotorCommandParameters, MotorState};
use kinematics::joints::{Joints, body::UpperBodyJoints};
use motionfile::{SplineInterpolator, TimedSpline};
use ros_z::prelude::*;

const ARM_JOINTS_TOPIC: &str = "arm_joints";
const PUBLISH_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    injected_arm_joints: Option<UpperBodyJoints>,
    motor_command_parameters: MotorCommandParameters,
    maximum_arm_joint_velocities: UpperBodyJoints,
}

#[derive(Default)]
struct ArmAnimator {
    latest_measured_positions: Option<UpperBodyJoints>,
    pending_target: Option<UpperBodyJoints>,
    active_transition: Option<SplineInterpolator<UpperBodyJoints>>,
    last_commanded_positions: Option<UpperBodyJoints>,
}

impl ArmAnimator {
    fn update_measured_positions(
        &mut self,
        measured_positions: Joints,
        maximum_arm_joint_velocities: UpperBodyJoints,
    ) -> Result<()> {
        self.latest_measured_positions = Some(measured_positions.into());

        if let Some(target) = self.pending_target.take() {
            self.start_transition(target, maximum_arm_joint_velocities)?;
        }

        Ok(())
    }

    fn set_target(
        &mut self,
        target: UpperBodyJoints,
        maximum_arm_joint_velocities: UpperBodyJoints,
    ) -> Result<()> {
        self.pending_target = Some(target);

        if self.latest_measured_positions.is_some() {
            let target = self
                .pending_target
                .take()
                .expect("target was just inserted");
            self.start_transition(target, maximum_arm_joint_velocities)?;
        }

        Ok(())
    }

    fn start_transition(
        &mut self,
        target: UpperBodyJoints,
        maximum_arm_joint_velocities: UpperBodyJoints,
    ) -> Result<()> {
        let current_positions = self
            .latest_measured_positions
            .expect("measured positions are required before starting a transition");
        let duration =
            transition_duration(current_positions, target, maximum_arm_joint_velocities)?;
        let spline = TimedSpline::try_new_transition_timed(current_positions, target, duration)
            .wrap_err("failed to create arm transition")?;

        self.active_transition = Some(SplineInterpolator::from(spline));
        self.last_commanded_positions
            .get_or_insert(current_positions);

        Ok(())
    }

    fn next_command(
        &mut self,
        time_step: Duration,
        maximum_arm_joint_velocities: UpperBodyJoints,
    ) -> Option<UpperBodyJoints> {
        let measured_positions = self.latest_measured_positions?;
        let transition = self.active_transition.as_mut()?;

        transition.advance_by(time_step);
        let desired_positions = transition.value();
        let previous_positions = self.last_commanded_positions.unwrap_or(measured_positions);
        let clamped_positions = clamp_joint_velocities(
            previous_positions,
            desired_positions,
            maximum_arm_joint_velocities,
            time_step,
        );

        self.last_commanded_positions = Some(clamped_positions);

        Some(clamped_positions)
    }
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("arm_animator").build().await?;

    let zenoh_session = ctx.session();

    let parameters = node.bind_parameter_as::<Parameters>("arm_animator")?;
    let arm_joints_sub = zenoh_session
        .declare_subscriber(ARM_JOINTS_TOPIC)
        .await
        .map_err(|error| eyre!("{error}"))?;
    let serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")?
        .build()
        .await?;
    let low_command_pub = node
        .publisher::<LowCommand>("commands/low_command")?
        .build()
        .await?;

    let mut publish_interval = tokio::time::interval(PUBLISH_INTERVAL);
    publish_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut arm_animator = ArmAnimator::default();
    let mut parameters_receiver = parameters.subscribe();

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();
        validate_maximum_arm_joint_velocities(parameters.maximum_arm_joint_velocities)?;

        tokio::select! {
            changed = parameters_receiver.changed() => {
                changed.wrap_err("arm_animator parameter watch ended")?;
                let parameters_snapshot = parameters_receiver.borrow_and_update().clone();
                let parameters = parameters_snapshot.typed();
                validate_maximum_arm_joint_velocities(parameters.maximum_arm_joint_velocities)?;

                if let Some(injected_arm_joints) = parameters.injected_arm_joints {
                    validate_arm_joint_positions(injected_arm_joints)?;
                    arm_animator.set_target(
                        injected_arm_joints,
                        parameters.maximum_arm_joint_velocities,
                    )?;
                }
            }
            arm_joints = arm_joints_sub.recv_async() => {
                let arm_joints = arm_joints.map_err(|error| eyre!("{error}"))?;
                let arm_joints: [f32; 8] = cdr::deserialize(&arm_joints.payload().to_bytes())
                    .wrap_err("failed to deserialize arm_joints")?;
                let upper_body_joints = UpperBodyJoints::from(arm_joints);
                validate_arm_joint_positions(upper_body_joints)?;

                arm_animator.set_target(
                    upper_body_joints,
                    parameters.maximum_arm_joint_velocities,
                )?;
            }
            serial_motor_states = serial_motor_states_sub.recv() => {
                let serial_motor_states = serial_motor_states?;
                arm_animator.update_measured_positions(
                    serial_motor_states.positions(),
                    parameters.maximum_arm_joint_velocities,
                )?;
            }
            _ = publish_interval.tick() => {
                let Some(target_upper_body_joints) = arm_animator.next_command(
                    PUBLISH_INTERVAL,
                    parameters.maximum_arm_joint_velocities,
                ) else {
                    warn!("skipping");
                    continue;
                };

                let target_joint_positions =
                    joints_from_upper_body_joints(target_upper_body_joints);
                let low_command = LowCommand::new(
                    &target_joint_positions,
                    &parameters.motor_command_parameters,
                    CommandType::Serial,
                );

                low_command_pub.publish(&low_command).await?;
            }
        }
    }
}

fn joints_from_upper_body_joints(upper_body_joints: UpperBodyJoints) -> Joints {
    Joints {
        left_arm: upper_body_joints.left_arm,
        right_arm: upper_body_joints.right_arm,
        ..Default::default()
    }
}

fn clamp_joint_velocities(
    previous: UpperBodyJoints,
    desired: UpperBodyJoints,
    maximum_velocities: UpperBodyJoints,
    time_step: Duration,
) -> UpperBodyJoints {
    let maximum_step = maximum_velocities * time_step.as_secs_f32();
    previous + (desired - previous).clamp(maximum_step * -1.0, maximum_step)
}

fn validate_maximum_arm_joint_velocities(maximum_velocities: UpperBodyJoints) -> Result<()> {
    for maximum_velocity in maximum_velocities {
        ensure!(
            maximum_velocity.is_finite() && maximum_velocity > 0.0,
            "maximum arm joint velocities must be positive finite values"
        );
    }

    Ok(())
}

fn validate_arm_joint_positions(arms: UpperBodyJoints) -> Result<()> {
    for position in arms {
        ensure!(
            position.is_finite(),
            "arm joint positions must be finite values"
        );
    }

    Ok(())
}

fn transition_duration(
    current_positions: UpperBodyJoints,
    target_positions: UpperBodyJoints,
    maximum_velocities: UpperBodyJoints,
) -> Result<Duration> {
    let mut maximum_duration = Duration::ZERO;

    for ((current_position, target_position), maximum_velocity) in current_positions
        .into_iter()
        .zip(target_positions)
        .zip(maximum_velocities)
    {
        let duration_seconds = ((target_position - current_position) / maximum_velocity).abs();
        let duration = Duration::try_from_secs_f32(duration_seconds)
            .wrap_err("arm transition duration must be representable")?;
        maximum_duration = maximum_duration.max(duration);
    }

    Ok(maximum_duration)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn maps_arm_array_to_left_and_right_arm_joints() {
        let arms = UpperBodyJoints::from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);

        assert_eq!(arms.left_arm.shoulder_pitch, 1.0);
        assert_eq!(arms.left_arm.shoulder_roll, 2.0);
        assert_eq!(arms.left_arm.shoulder_yaw, 3.0);
        assert_eq!(arms.left_arm.elbow, 4.0);
        assert_eq!(arms.right_arm.shoulder_pitch, 5.0);
        assert_eq!(arms.right_arm.shoulder_roll, 6.0);
        assert_eq!(arms.right_arm.shoulder_yaw, 7.0);
        assert_eq!(arms.right_arm.elbow, 8.0);
    }

    #[test]
    fn builds_joints_with_upper_body_positions_and_zero_rest() {
        let upper_body_joints =
            UpperBodyJoints::from([10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0]);

        let joints = joints_from_upper_body_joints(upper_body_joints);

        assert_eq!(joints.head, Default::default());
        assert_eq!(joints.left_leg, Default::default());
        assert_eq!(joints.right_leg, Default::default());
        assert_eq!(joints.left_arm, upper_body_joints.left_arm);
        assert_eq!(joints.right_arm, upper_body_joints.right_arm);
    }

    #[test]
    fn clamp_joint_velocities_limits_each_joint_step() {
        let previous = UpperBodyJoints::from([0.0; 8]);
        let desired = UpperBodyJoints::from([1.0, -1.0, 0.01, -0.01, 2.0, -2.0, 0.05, -0.05]);
        let maximum_velocities = UpperBodyJoints::from([2.0, 2.0, 2.0, 2.0, 5.0, 5.0, 5.0, 5.0]);

        let clamped = clamp_joint_velocities(
            previous,
            desired,
            maximum_velocities,
            Duration::from_millis(20),
        );

        assert_close(clamped.left_arm.shoulder_pitch, 0.04);
        assert_close(clamped.left_arm.shoulder_roll, -0.04);
        assert_close(clamped.left_arm.shoulder_yaw, 0.01);
        assert_close(clamped.left_arm.elbow, -0.01);
        assert_close(clamped.right_arm.shoulder_pitch, 0.1);
        assert_close(clamped.right_arm.shoulder_roll, -0.1);
        assert_close(clamped.right_arm.shoulder_yaw, 0.05);
        assert_close(clamped.right_arm.elbow, -0.05);
    }

    #[test]
    fn retargeting_preserves_last_commanded_arms_for_velocity_clamp_baseline() {
        let mut animator = ArmAnimator::default();
        let measured_positions = Joints::default();
        let maximum_velocities = UpperBodyJoints::from([1.0; 8]);
        let last_commanded_positions = UpperBodyJoints::from([0.5; 8]);

        animator
            .update_measured_positions(measured_positions, maximum_velocities)
            .unwrap();
        animator.last_commanded_positions = Some(last_commanded_positions);

        animator
            .set_target(UpperBodyJoints::from([1.0; 8]), maximum_velocities)
            .unwrap();

        assert_eq!(
            animator.last_commanded_positions,
            Some(last_commanded_positions)
        );
    }

    #[test]
    fn rejects_unrepresentable_transition_durations_without_panicking() {
        let mut animator = ArmAnimator::default();
        animator
            .update_measured_positions(Joints::default(), UpperBodyJoints::from([1.0; 8]))
            .unwrap();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            animator.set_target(
                UpperBodyJoints::from([f32::MAX; 8]),
                UpperBodyJoints::from([f32::MIN_POSITIVE; 8]),
            )
        }));

        assert!(result.is_ok(), "transition validation should not panic");
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn validates_positive_finite_arm_velocity_limits() {
        assert!(validate_maximum_arm_joint_velocities(UpperBodyJoints::from([1.0; 8])).is_ok());
        assert!(
            validate_maximum_arm_joint_velocities(UpperBodyJoints::from([
                1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]))
            .is_err()
        );
        assert!(
            validate_maximum_arm_joint_velocities(UpperBodyJoints::from([
                1.0,
                f32::NAN,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            ]))
            .is_err()
        );
    }

    #[test]
    fn validates_finite_arm_target_positions() {
        assert!(validate_arm_joint_positions(UpperBodyJoints::from([0.0; 8])).is_ok());
        assert!(
            validate_arm_joint_positions(UpperBodyJoints::from([
                0.0,
                f32::NAN,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ]))
            .is_err()
        );
        assert!(
            validate_arm_joint_positions(UpperBodyJoints::from([
                0.0,
                f32::INFINITY,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ]))
            .is_err()
        );
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }
}
