use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::{
    Result,
    eyre::{Context as _, ensure, eyre},
};
use serde::{Deserialize, Serialize};
use tokio::time::MissedTickBehavior;

use booster::{CommandType, JointsMotorState, LowCommand, MotorCommandParameters, MotorState};
use kinematics::joints::{Joints, arm::ArmJoints};
use motionfile::{SplineInterpolator, TimedSpline};
use ros_z::{message::WireEncoder, prelude::*};

const ARM_JOINTS_TOPIC: &str = "arm_joints";
const JOINT_CTRL_TOPIC: &str = "rt/joint_ctrl";
const PUBLISH_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    motor_command_parameters: MotorCommandParameters,
    maximum_arm_joint_velocities: BothArms,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Message)]
struct BothArms {
    left_arm: ArmJoints,
    right_arm: ArmJoints,
}

impl From<[f32; 8]> for BothArms {
    fn from(values: [f32; 8]) -> Self {
        BothArms {
            left_arm: ArmJoints {
                shoulder_pitch: values[0],
                shoulder_roll: values[1],
                shoulder_yaw: values[2],
                elbow: values[3],
            },
            right_arm: ArmJoints {
                shoulder_pitch: values[4],
                shoulder_roll: values[5],
                shoulder_yaw: values[6],
                elbow: values[7],
            },
        }
    }
}

impl From<Joints> for BothArms {
    fn from(joints: Joints) -> Self {
        Self {
            left_arm: joints.left_arm,
            right_arm: joints.right_arm,
        }
    }
}

impl BothArms {
    fn values(self) -> [f32; 8] {
        [
            self.left_arm.shoulder_pitch,
            self.left_arm.shoulder_roll,
            self.left_arm.shoulder_yaw,
            self.left_arm.elbow,
            self.right_arm.shoulder_pitch,
            self.right_arm.shoulder_roll,
            self.right_arm.shoulder_yaw,
            self.right_arm.elbow,
        ]
    }
}

#[derive(Default)]
struct ArmAnimator {
    latest_measured_positions: Option<Joints>,
    pending_target: Option<BothArms>,
    active_transition: Option<SplineInterpolator<Joints>>,
    last_commanded_arms: Option<BothArms>,
}

impl ArmAnimator {
    fn update_measured_positions(
        &mut self,
        measured_positions: Joints,
        maximum_arm_joint_velocities: BothArms,
    ) -> Result<()> {
        self.latest_measured_positions = Some(measured_positions);

        if let Some(target) = self.pending_target.take() {
            self.start_transition(target, maximum_arm_joint_velocities)?;
        }

        Ok(())
    }

    fn set_target(
        &mut self,
        target: BothArms,
        maximum_arm_joint_velocities: BothArms,
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
        target: BothArms,
        maximum_arm_joint_velocities: BothArms,
    ) -> Result<()> {
        let current_positions = self
            .latest_measured_positions
            .expect("measured positions are required before starting a transition");
        let target_positions = apply_arms(current_positions, target);
        let maximum_velocities =
            apply_arms(Joints::fill(f32::INFINITY), maximum_arm_joint_velocities);
        validate_transition_durations(current_positions, target_positions, maximum_velocities)?;
        let spline = TimedSpline::try_new_transition_with_velocity(
            current_positions,
            target_positions,
            maximum_velocities,
        )
        .wrap_err("failed to create arm transition")?;

        self.active_transition = Some(SplineInterpolator::from(spline));
        self.last_commanded_arms
            .get_or_insert_with(|| current_positions.into());

        Ok(())
    }

    fn next_command(
        &mut self,
        time_step: Duration,
        maximum_arm_joint_velocities: BothArms,
    ) -> Option<Joints> {
        let measured_positions = self.latest_measured_positions?;
        let transition = self.active_transition.as_mut()?;

        transition.advance_by(time_step);
        let desired_positions = transition.value();
        let previous_arms = self
            .last_commanded_arms
            .unwrap_or_else(|| measured_positions.into());
        let clamped_arms = clamp_arm_velocities(
            previous_arms,
            desired_positions.into(),
            maximum_arm_joint_velocities,
            time_step,
        );

        self.last_commanded_arms = Some(clamped_arms);

        Some(apply_arms(measured_positions, clamped_arms))
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

    let zenoh_publisher = zenoh_session
        .declare_publisher(JOINT_CTRL_TOPIC)
        .await
        .map_err(|error| eyre!("{error}"))?;
    let mut publish_interval = tokio::time::interval(PUBLISH_INTERVAL);
    publish_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut arm_animator = ArmAnimator::default();

    loop {
        let parameters = parameters.snapshot().typed().clone();
        validate_maximum_arm_joint_velocities(parameters.maximum_arm_joint_velocities)?;

        tokio::select! {
            arm_joints = arm_joints_sub.recv_async() => {
                let arm_joints = arm_joints.map_err(|error| eyre!("{error}"))?;
                let arm_joints: [f32; 8] = cdr::deserialize(&arm_joints.payload().to_bytes())
                    .wrap_err("failed to deserialize arm_joints")?;
                let both_arms: BothArms = arm_joints.into();
                validate_arm_joint_positions(both_arms)?;

                arm_animator.set_target(
                    both_arms,
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
                let Some(target_joint_positions) = arm_animator.next_command(
                    PUBLISH_INTERVAL,
                    parameters.maximum_arm_joint_velocities,
                ) else {
                    continue;
                };

                let low_command = LowCommand::new(
                    &target_joint_positions,
                    &parameters.motor_command_parameters,
                    CommandType::Serial,
                );

                low_command_pub.publish(&low_command).await?;

                let low_command_bytes = <LowCommand as Message>::Codec::serialize(&low_command)?;

                zenoh_publisher
                    .put(&low_command_bytes)
                    .await
                    .map_err(|error| eyre!("{error}"))?;
            }
        }
    }
}

fn apply_arms(mut joints: Joints, arms: BothArms) -> Joints {
    joints.left_arm = arms.left_arm;
    joints.right_arm = arms.right_arm;
    joints
}

fn clamp_arm_velocities(
    previous: BothArms,
    desired: BothArms,
    maximum_velocities: BothArms,
    time_step: Duration,
) -> BothArms {
    BothArms {
        left_arm: clamp_arm_joints(
            previous.left_arm,
            desired.left_arm,
            maximum_velocities.left_arm,
            time_step,
        ),
        right_arm: clamp_arm_joints(
            previous.right_arm,
            desired.right_arm,
            maximum_velocities.right_arm,
            time_step,
        ),
    }
}

fn clamp_arm_joints(
    previous: ArmJoints,
    desired: ArmJoints,
    maximum_velocities: ArmJoints,
    time_step: Duration,
) -> ArmJoints {
    ArmJoints {
        shoulder_pitch: clamp_joint_velocity(
            previous.shoulder_pitch,
            desired.shoulder_pitch,
            maximum_velocities.shoulder_pitch,
            time_step,
        ),
        shoulder_roll: clamp_joint_velocity(
            previous.shoulder_roll,
            desired.shoulder_roll,
            maximum_velocities.shoulder_roll,
            time_step,
        ),
        shoulder_yaw: clamp_joint_velocity(
            previous.shoulder_yaw,
            desired.shoulder_yaw,
            maximum_velocities.shoulder_yaw,
            time_step,
        ),
        elbow: clamp_joint_velocity(
            previous.elbow,
            desired.elbow,
            maximum_velocities.elbow,
            time_step,
        ),
    }
}

fn clamp_joint_velocity(
    previous: f32,
    desired: f32,
    maximum_velocity: f32,
    time_step: Duration,
) -> f32 {
    let maximum_step = maximum_velocity * time_step.as_secs_f32();
    previous + (desired - previous).clamp(-maximum_step, maximum_step)
}

fn validate_maximum_arm_joint_velocities(maximum_velocities: BothArms) -> Result<()> {
    for maximum_velocity in maximum_velocities.values() {
        ensure!(
            maximum_velocity.is_finite() && maximum_velocity > 0.0,
            "maximum arm joint velocities must be positive finite values"
        );
    }

    Ok(())
}

fn validate_arm_joint_positions(arms: BothArms) -> Result<()> {
    for position in arms.values() {
        ensure!(
            position.is_finite(),
            "arm joint positions must be finite values"
        );
    }

    Ok(())
}

fn validate_transition_durations(
    current_positions: Joints,
    target_positions: Joints,
    maximum_velocities: Joints,
) -> Result<()> {
    for ((current_position, target_position), maximum_velocity) in current_positions
        .into_iter()
        .zip(target_positions)
        .zip(maximum_velocities)
    {
        let duration_seconds = ((target_position - current_position) / maximum_velocity).abs();
        ensure!(
            Duration::try_from_secs_f32(duration_seconds).is_ok(),
            "arm transition duration must be representable"
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
        let arms = BothArms::from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);

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
    fn apply_arms_preserves_non_arm_joints() {
        let measured_positions = numbered_joints();
        let arms = BothArms::from([10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0]);

        let target_positions = apply_arms(measured_positions, arms);

        assert_eq!(target_positions.head, measured_positions.head);
        assert_eq!(target_positions.left_leg, measured_positions.left_leg);
        assert_eq!(target_positions.right_leg, measured_positions.right_leg);
        assert_eq!(target_positions.left_arm, arms.left_arm);
        assert_eq!(target_positions.right_arm, arms.right_arm);
    }

    #[test]
    fn clamp_arm_velocities_limits_each_joint_step() {
        let previous = BothArms::from([0.0; 8]);
        let desired = BothArms::from([1.0, -1.0, 0.01, -0.01, 2.0, -2.0, 0.05, -0.05]);
        let maximum_velocities = BothArms::from([2.0, 2.0, 2.0, 2.0, 5.0, 5.0, 5.0, 5.0]);

        let clamped = clamp_arm_velocities(
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
        let maximum_velocities = BothArms::from([1.0; 8]);
        let last_commanded_arms = BothArms::from([0.5; 8]);

        animator
            .update_measured_positions(measured_positions, maximum_velocities)
            .unwrap();
        animator.last_commanded_arms = Some(last_commanded_arms);

        animator
            .set_target(BothArms::from([1.0; 8]), maximum_velocities)
            .unwrap();

        assert_eq!(animator.last_commanded_arms, Some(last_commanded_arms));
    }

    #[test]
    fn rejects_unrepresentable_transition_durations_without_panicking() {
        let mut animator = ArmAnimator::default();
        animator
            .update_measured_positions(Joints::default(), BothArms::from([1.0; 8]))
            .unwrap();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            animator.set_target(
                BothArms::from([f32::MAX; 8]),
                BothArms::from([f32::MIN_POSITIVE; 8]),
            )
        }));

        assert!(result.is_ok(), "transition validation should not panic");
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn validates_positive_finite_arm_velocity_limits() {
        assert!(validate_maximum_arm_joint_velocities(BothArms::from([1.0; 8])).is_ok());
        assert!(
            validate_maximum_arm_joint_velocities(BothArms::from([
                1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ]))
            .is_err()
        );
        assert!(
            validate_maximum_arm_joint_velocities(BothArms::from([
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
        assert!(validate_arm_joint_positions(BothArms::from([0.0; 8])).is_ok());
        assert!(
            validate_arm_joint_positions(BothArms::from([
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
            validate_arm_joint_positions(BothArms::from([
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

    fn numbered_joints() -> Joints {
        (0..22).map(|value| value as f32).collect()
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }
}
