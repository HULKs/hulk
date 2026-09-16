use std::{pin::Pin, sync::Arc, time::Duration};

use color_eyre::{
    Result,
    eyre::{WrapErr, ensure, eyre},
};
use serde::{Deserialize, Serialize};

use head_motion::node::{HEAD_MOTION_SERVICE_TOPIC, HeadMotionService};
use kinematics::joints::{
    Joints,
    body::{BodyJoints, LowerBodyJoints, UpperBodyJoints},
    leg::LegJoints,
};
use linear_algebra::vector;
use motion_inference::{
    inference::{
        GetUpCommand, InferenceCommand, InferenceRequest, InferenceResponse, KickCommand,
        PolicyExecution, WalkCommand, joints_are_finite,
    },
    locomotion::{KickRequest, leg},
    node::{
        GETUP_INFERENCE_SERVICE, GetUpInferenceService, InferenceResult, KICK_INFERENCE_SERVICE,
        KickInferenceService, WALK_INFERENCE_SERVICE, WalkInferenceService,
    },
};
use ros_z::{
    Message,
    context::Context,
    parameter::NodeParametersExt,
    qos::{QosDurability, QosProfile},
    service::ServiceClient,
    time::Clock,
};
use types::{
    joint_limits::JointLimits,
    motion_command::{HeadMotion, MotionCommand},
    motor_command::MotorCommand,
    time_wrapper::TimeWrapper,
};

use crate::{
    command::RobotCommand,
    walking::{WalkingParameters, step_from_walk_command},
};

pub mod command;
mod inputs;
mod node;
mod recovery;
pub mod walking;

pub const ROBOT_COMMAND_TOPIC: &str = "commands/robot_command";

#[derive(Serialize, Deserialize, Message)]
struct ArmParameters {
    arm_blend_duration: Duration,

    shoulder_pitch_scale: f32,
    shoulder_roll_degrees: f32,
    shoulder_roll_scale: f32,
    knee_lateral_offset: f32,
    elbow_degrees: f32,
    elbow_scale: f32,

    kp: f32,
    kd: f32,
}

#[derive(Serialize, Deserialize, Message)]
struct Parameters {
    arms: ArmParameters,
    walking: WalkingParameters,
    inference_timeout: Duration,
    head_motion_timeout: Duration,
    maximum_command_age: Duration,
    maximum_sensor_age: Duration,
    maximum_hardware_age: Duration,
    recovery: recovery::RecoveryParameters,
}

impl Parameters {
    fn validate(&self) -> std::result::Result<(), String> {
        self.recovery.validate()?;
        let a = &self.arms;
        let w = &self.walking;
        if [
            a.shoulder_pitch_scale,
            a.shoulder_roll_degrees,
            a.shoulder_roll_scale,
            a.knee_lateral_offset,
            a.elbow_degrees,
            a.elbow_scale,
        ]
        .into_iter()
        .any(|v| !v.is_finite())
            || [a.kp, a.kd, w.max_alignment_rate]
                .into_iter()
                .any(|v| !v.is_finite() || v < 0.0)
            || [w.hybrid_align_distance, w.deceleration_distance]
                .into_iter()
                .any(|v| !v.is_finite() || v <= 0.0)
            || [
                a.arm_blend_duration,
                self.inference_timeout,
                self.head_motion_timeout,
                self.maximum_command_age,
                self.maximum_sensor_age,
                self.maximum_hardware_age,
            ]
            .into_iter()
            .any(|v| v.is_zero())
        {
            return Err("invalid motion coefficients, gains, or durations".into());
        }
        Ok(())
    }
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(node::run(ctx))
}

struct MotionState {
    head_motion_client: ServiceClient<HeadMotionService>,
    walk_inference_client: ServiceClient<WalkInferenceService>,
    kick_inference_client: ServiceClient<KickInferenceService>,
    get_up_inference_client: ServiceClient<GetUpInferenceService>,
    generation: u64,
    active: bool,
    last_policy: Option<PolicyExecution>,
    last_arms: TimeWrapper<UpperBodyJoints<f32>>,
}

enum MotionPlan {
    Damping,
    Prepare,
    GetUp {
        command: GetUpCommand,
    },
    Walk {
        head_motion: HeadMotion,
        command: WalkCommand,
    },
    Kick {
        head_motion: HeadMotion,
        command: KickCommand,
    },
}

impl MotionPlan {
    fn from_motion_command(
        motion_command: &MotionCommand,
        parameters: &WalkingParameters,
    ) -> Result<Self> {
        Ok(match motion_command {
            MotionCommand::Damping => Self::Damping,
            MotionCommand::Prepare => Self::Prepare,
            MotionCommand::Stand { head } => Self::Walk {
                head_motion: *head,
                command: WalkCommand::stand(),
            },
            MotionCommand::StandUp { fast } => Self::GetUp {
                command: GetUpCommand { fast: *fast },
            },
            MotionCommand::Kick {
                head,
                ball_position,
                ball_velocity,
                target_speed,
                soft,
                quick,
                kick_direction,
                strong,
            } => Self::Kick {
                head_motion: *head,
                command: KickCommand {
                    soft: *soft,
                    request: KickRequest {
                        ball_position: *ball_position,
                        ball_velocity: *ball_velocity,
                        // TODO: use timestamped odometry to compensate stale ball coordinates
                        // and kick direction for robot motion before inference.
                        direction: kick_direction.angle(),
                        target_speed: *target_speed,
                        strong: *strong,
                        quick: *quick,
                    },
                },
            },
            MotionCommand::Walk {
                head,
                path,
                orientation_mode,
                target_orientation,
                distance_to_be_aligned,
                speed,
            } => {
                let step = step_from_walk_command(
                    path,
                    *orientation_mode,
                    *target_orientation,
                    *distance_to_be_aligned,
                    *speed,
                    parameters,
                )?;

                Self::Walk {
                    head_motion: *head,
                    command: WalkCommand {
                        velocity: vector![step.forward, step.left],
                        angular_velocity: step.turn,
                    },
                }
            }
            MotionCommand::WalkWithVelocity {
                head,
                velocity,
                angular_velocity,
            } => Self::Walk {
                head_motion: *head,
                command: WalkCommand {
                    velocity: *velocity,
                    angular_velocity: *angular_velocity,
                },
            },
        })
    }
}

impl MotionState {
    fn deactivate(&mut self) {
        if self.active {
            self.generation = self.generation.saturating_add(1);
        }
        self.active = false;
        self.last_policy = None;
    }

    async fn infer(
        &mut self,
        motion_plan: MotionPlan,
        clock: &Clock,
        parameters: &Parameters,
        joint_limits: &JointLimits,
    ) -> Result<RobotCommand> {
        match motion_plan {
            MotionPlan::Damping => {
                self.deactivate();
                return Ok(RobotCommand::Damping);
            }
            MotionPlan::Prepare => {
                self.deactivate();
                return Ok(RobotCommand::Prepare);
            }
            _ => {}
        }
        if !self.active {
            self.generation = self.generation.saturating_add(1);
            self.active = true;
        }
        let now = clock.now();
        let (joints_command, execution) = self
            .infer_policy(motion_plan, now, clock, parameters, joint_limits)
            .await?;
        ensure!(
            clock.now() < now + parameters.inference_timeout,
            "motion inference deadline expired before dispatch"
        );
        let robot_command = RobotCommand::Custom { joints_command }.clamp(joint_limits)?;
        if let RobotCommand::Custom { joints_command } = &robot_command {
            self.last_arms = TimeWrapper {
                time: now,
                inner: joints_command.upper_body_as_ref().map(|j| j.position),
            };
        }
        self.last_policy = Some(execution);
        Ok(robot_command)
    }

    fn request<C>(
        &self,
        command: C,
        now: ros_z::time::Time,
        timeout: Duration,
    ) -> InferenceRequest<C> {
        InferenceRequest {
            generation: self.generation,
            requested_at: now,
            valid_until: now + timeout,
            command,
        }
    }

    async fn infer_policy(
        &self,
        plan: MotionPlan,
        now: ros_z::time::Time,
        clock: &Clock,
        p: &Parameters,
        limits: &JointLimits,
    ) -> Result<(Joints<MotorCommand>, PolicyExecution)> {
        match plan {
            MotionPlan::Walk {
                command,
                head_motion,
            } => {
                let request = self.request(command, now, p.inference_timeout);
                let body = self
                    .walk_inference_client
                    .call_with_timeout_async(&request, p.inference_timeout);
                self.infer_lower_body(
                    body,
                    head_motion,
                    InferenceCommand::Walk(command).policy(),
                    clock,
                    p,
                    limits,
                )
                .await
            }
            MotionPlan::Kick {
                command,
                head_motion,
            } => {
                let request = self.request(command, now, p.inference_timeout);
                let body = self
                    .kick_inference_client
                    .call_with_timeout_async(&request, p.inference_timeout);
                self.infer_lower_body(
                    body,
                    head_motion,
                    InferenceCommand::Kick(command).policy(),
                    clock,
                    p,
                    limits,
                )
                .await
            }
            MotionPlan::GetUp { command } => {
                let request = self.request(command, now, p.inference_timeout);
                let output = self
                    .get_up_inference_client
                    .call_with_timeout_async(&request, p.inference_timeout)
                    .await??;
                ensure!(
                    output.execution.policy == InferenceCommand::GetUp(command).policy(),
                    "inference returned the wrong policy"
                );
                Ok((*output.joints, output.execution))
            }
            MotionPlan::Damping | MotionPlan::Prepare => Err(eyre!("inference requires a policy")),
        }
    }

    async fn infer_lower_body(
        &self,
        body: impl std::future::Future<
            Output = ros_z::Result<
                InferenceResult<InferenceResponse<LowerBodyJoints<MotorCommand>>>,
            >,
        >,
        head: HeadMotion,
        policy: motion_inference::config::Policy,
        clock: &Clock,
        p: &Parameters,
        limits: &JointLimits,
    ) -> Result<(Joints<MotorCommand>, PolicyExecution)> {
        let head = self
            .head_motion_client
            .call_with_timeout_async(&head, p.head_motion_timeout);
        let (output, head) = tokio::join!(body, head);
        let output = output??;
        let head = head?;
        ensure!(
            output.execution.policy == policy,
            "inference returned the wrong policy"
        );
        let legs = *output.joints;
        let arms = self.generate_walking_arm_joints(&legs, clock, &p.arms, limits)?;
        Ok((
            Joints::from_head_and_body(head, BodyJoints::from_lower_and_upper(legs, arms)),
            output.execution,
        ))
    }

    fn generate_walking_arm_joints(
        &self,
        legs: &LowerBodyJoints<MotorCommand>,
        clock: &Clock,
        parameters: &ArmParameters,
        joint_limits: &JointLimits,
    ) -> Result<UpperBodyJoints<MotorCommand>> {
        let elapsed = clock.now().duration_since(self.last_arms.time);

        let legs: LowerBodyJoints = LowerBodyJoints {
            left_leg: LegJoints {
                hip_pitch: legs.left_leg.hip_pitch.position.clamp(
                    joint_limits.position.left_leg.hip_pitch[0],
                    joint_limits.position.left_leg.hip_pitch[1],
                ),
                hip_roll: legs.left_leg.hip_roll.position.clamp(
                    joint_limits.position.left_leg.hip_roll[0],
                    joint_limits.position.left_leg.hip_roll[1],
                ),
                hip_yaw: legs.left_leg.hip_yaw.position.clamp(
                    joint_limits.position.left_leg.hip_yaw[0],
                    joint_limits.position.left_leg.hip_yaw[1],
                ),
                knee: legs.left_leg.knee.position.clamp(
                    joint_limits.position.left_leg.knee[0],
                    joint_limits.position.left_leg.knee[1],
                ),
                ankle_up: legs.left_leg.ankle_up.position.clamp(
                    joint_limits.position.left_leg.ankle_up[0],
                    joint_limits.position.left_leg.ankle_up[1],
                ),
                ankle_down: legs.left_leg.ankle_down.position.clamp(
                    joint_limits.position.left_leg.ankle_down[0],
                    joint_limits.position.left_leg.ankle_down[1],
                ),
            },
            right_leg: LegJoints {
                hip_pitch: legs.right_leg.hip_pitch.position.clamp(
                    joint_limits.position.right_leg.hip_pitch[0],
                    joint_limits.position.right_leg.hip_pitch[1],
                ),
                hip_roll: legs.right_leg.hip_roll.position.clamp(
                    joint_limits.position.right_leg.hip_roll[0],
                    joint_limits.position.right_leg.hip_roll[1],
                ),
                hip_yaw: legs.right_leg.hip_yaw.position.clamp(
                    joint_limits.position.right_leg.hip_yaw[0],
                    joint_limits.position.right_leg.hip_yaw[1],
                ),
                knee: legs.right_leg.knee.position.clamp(
                    joint_limits.position.right_leg.knee[0],
                    joint_limits.position.right_leg.knee[1],
                ),
                ankle_up: legs.right_leg.ankle_up.position.clamp(
                    joint_limits.position.right_leg.ankle_up[0],
                    joint_limits.position.right_leg.ankle_up[1],
                ),
                ankle_down: legs.right_leg.ankle_down.position.clamp(
                    joint_limits.position.right_leg.ankle_down[0],
                    joint_limits.position.right_leg.ankle_down[1],
                ),
            },
        };

        let ratio =
            (elapsed.as_secs_f32() / parameters.arm_blend_duration.as_secs_f32()).clamp(0.0, 1.0);
        let mut target = Joints::fill(0.0);
        for (left, arm, leg_angles, initial, sign) in [
            (
                true,
                &mut target.left_arm,
                &legs.left_leg,
                self.last_arms.inner.left_arm,
                1.0,
            ),
            (
                false,
                &mut target.right_arm,
                &legs.right_leg,
                self.last_arms.inner.right_arm,
                -1.0,
            ),
        ] {
            let (sole, knee) = leg(leg_angles, left);
            arm.shoulder_pitch = sole.x() * parameters.shoulder_pitch_scale;
            arm.shoulder_roll = sign
                * (parameters.shoulder_roll_degrees.to_radians()
                    + (sign * knee.y() - parameters.knee_lateral_offset).max(0.0)
                        * parameters.shoulder_roll_scale);
            arm.shoulder_yaw = 0.0;
            arm.elbow = sign
                * (parameters.elbow_degrees.to_radians()
                    + sole.x() * parameters.shoulder_pitch_scale * parameters.elbow_scale);
            *arm = initial * (1.0 - ratio) + *arm * ratio;
        }
        // let joints = position_targets(target, parameters.kp, parameters.kd);
        let joints = target.map(|position| MotorCommand {
            position,
            kp: parameters.kp,
            kd: parameters.kd,
            ..MotorCommand::zeros()
        });
        ensure!(
            joints_are_finite(&joints),
            "non-finite generated arm joints"
        );
        Ok(UpperBodyJoints {
            left_arm: joints.left_arm,
            right_arm: joints.right_arm,
        })
    }
}
