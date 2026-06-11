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
use ros_z::prelude::*;
use types::motion_command::MotionCommand;

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
    target: Option<UpperBodyJoints>,
    last_commanded_positions: Option<UpperBodyJoints>,
    commands_active: bool,
}

impl ArmAnimator {
    fn update_measured_positions(&mut self, measured_positions: Joints) {
        self.latest_measured_positions = Some(measured_positions.into());

        if self.commands_active && self.last_commanded_positions.is_none() {
            self.last_commanded_positions = self.latest_measured_positions;
        }
    }

    fn set_target(&mut self, target: UpperBodyJoints) {
        self.target = Some(target);
    }

    fn set_commands_active(&mut self, commands_active: bool) {
        if self.commands_active == commands_active {
            return;
        }

        self.commands_active = commands_active;

        if commands_active {
            self.last_commanded_positions = self.latest_measured_positions;
        } else {
            self.last_commanded_positions = None;
        }
    }

    fn commands_active(&self) -> bool {
        self.commands_active
    }

    fn next_command(
        &mut self,
        time_step: Duration,
        maximum_arm_joint_velocities: UpperBodyJoints,
    ) -> Option<UpperBodyJoints> {
        if !self.commands_active {
            return None;
        }

        let measured_positions = self.latest_measured_positions?;
        let desired_positions = self.target?;
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
    let motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
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
                    arm_animator.set_target(injected_arm_joints);
                }
            }
            arm_joints = arm_joints_sub.recv_async(), if arm_animator.commands_active() => {
                let arm_joints = arm_joints.map_err(|error| eyre!("{error}"))?;
                let arm_joints: [f32; 8] = cdr::deserialize(&arm_joints.payload().to_bytes())
                    .wrap_err("failed to deserialize arm_joints")?;
                let upper_body_joints = UpperBodyJoints::from(arm_joints);
                validate_arm_joint_positions(upper_body_joints)?;

                arm_animator.set_target(upper_body_joints);
            }
            serial_motor_states = serial_motor_states_sub.recv() => {
                let serial_motor_states = serial_motor_states?;
                arm_animator.update_measured_positions(serial_motor_states.positions());
            }
            motion_command = motion_command_sub.recv() => {
                let motion_command = motion_command?;
                arm_animator.set_commands_active(
                    matches!(motion_command, MotionCommand::Custom),
                );
            }
            _ = publish_interval.tick() => {
                if !arm_animator.commands_active() {
                    continue;
                }

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
    fn target_waits_until_commands_are_active() {
        let mut animator = ArmAnimator::default();
        let maximum_velocities = UpperBodyJoints::from([1.0; 8]);

        animator.update_measured_positions(Joints::default());
        animator.set_target(UpperBodyJoints::from([1.0; 8]));

        assert!(
            animator
                .next_command(Duration::from_millis(20), maximum_velocities)
                .is_none()
        );

        animator.set_commands_active(true);

        assert!(
            animator
                .next_command(Duration::from_millis(20), maximum_velocities)
                .is_some()
        );
    }

    #[test]
    fn deactivating_commands_stops_command_output() {
        let mut animator = ArmAnimator::default();
        let maximum_velocities = UpperBodyJoints::from([1.0; 8]);

        animator.update_measured_positions(Joints::default());
        animator.set_target(UpperBodyJoints::from([1.0; 8]));
        animator.set_commands_active(true);

        assert!(
            animator
                .next_command(Duration::from_millis(20), maximum_velocities)
                .is_some()
        );
        animator.set_commands_active(false);

        assert!(animator.last_commanded_positions.is_none());
        assert!(
            animator
                .next_command(Duration::from_millis(20), maximum_velocities)
                .is_none()
        );
    }

    #[test]
    fn first_active_command_is_clamped_against_latest_measured_positions() {
        let mut animator = ArmAnimator::default();
        let maximum_velocities = UpperBodyJoints::from([1.0; 8]);

        animator.update_measured_positions(Joints::default());
        animator.set_target(UpperBodyJoints::from([1.0; 8]));
        animator.set_commands_active(true);

        let first_command = animator
            .next_command(Duration::from_millis(500), maximum_velocities)
            .unwrap();

        assert!(first_command.left_arm.shoulder_pitch <= 0.500_001);
        assert!(first_command.left_arm.shoulder_pitch > 0.1);
    }

    #[test]
    fn subsequent_commands_advance_from_last_commanded_baseline() {
        let mut animator = ArmAnimator::default();
        let maximum_velocities = UpperBodyJoints::from([1.0; 8]);

        animator.update_measured_positions(Joints::default());
        animator.set_target(UpperBodyJoints::from([1.0; 8]));
        animator.set_commands_active(true);

        let first_command = animator
            .next_command(Duration::from_millis(500), maximum_velocities)
            .unwrap();
        animator.update_measured_positions(Joints::default());
        let second_command = animator
            .next_command(Duration::from_millis(20), maximum_velocities)
            .unwrap();

        assert!(second_command.left_arm.shoulder_pitch > first_command.left_arm.shoulder_pitch);
    }

    #[test]
    fn animation_samples_are_forwarded_as_desired_positions() {
        let mut animator = ArmAnimator::default();
        let maximum_velocities = UpperBodyJoints::from([100.0; 8]);

        animator.update_measured_positions(Joints::default());
        animator.set_commands_active(true);
        animator.set_target(UpperBodyJoints::from([0.2; 8]));

        let command = animator
            .next_command(Duration::from_millis(20), maximum_velocities)
            .unwrap();

        assert_close(command.left_arm.shoulder_pitch, 0.2);

        animator.set_target(UpperBodyJoints::from([0.4; 8]));

        let command = animator
            .next_command(Duration::from_millis(20), maximum_velocities)
            .unwrap();

        assert_close(command.left_arm.shoulder_pitch, 0.4);
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
