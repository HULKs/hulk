use std::{fmt::Display, pin::Pin, sync::Arc, time::Duration};

use booster::MotorState;
use color_eyre::{
    Result,
    eyre::{WrapErr, ensure, eyre},
};
use coordinate_systems::Ground;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use head_motion::node::{HEAD_MOTION_SERVICE_TOPIC, HeadMotionService};
use kinematics::joints::{
    Joints,
    body::{BodyJoints, LowerBodyJoints, UpperBodyJoints},
    head::HeadJoints,
};
use linear_algebra::{Vector2, vector};
use motion_inference::{
    inference::{GetUpCommand, KickCommand, WalkCommand, joints_are_finite},
    locomotion::{KickRequest, leg},
    node::{
        GETUP_INFERENCE_SERVICE, GetUpInferenceService, InferenceRequest, KICK_INFERENCE_SERVICE,
        KickInferenceService, WALK_INFERENCE_SERVICE, WalkInferenceService,
    },
};
use ros_z::{
    Message, Service,
    context::Context,
    node::Node,
    parameter::NodeParametersExt,
    pubsub::Publisher,
    qos::{QosDurability, QosProfile, QosReliability},
    service::ServiceClient,
    time::{Clock, Time},
};
use types::{
    joint_limits::JointLimits,
    motion_command::{HeadMotion, MotionCommand},
    motor_command::MotorCommand,
    robot_command::RobotCommand,
    walking_velocity_limits::{WALKING_VELOCITY_LIMITS_TOPIC, WalkingVelocityLimits},
};

use crate::walking::{WalkingParameters, step_from_walk_command};

pub mod walking;

const ROBOT_COMMAND_TOPIC: &str = "commands/robot_command";

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
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node: Arc<Node> = Arc::new(
        ctx.create_node("motion")
            .build()
            .await
            .wrap_err("failed to create motion node")?,
    );

    let parameters = node.bind_parameter_as::<Parameters>("motion")?;

    let motion_command_cache = node
        .subscriber::<MotionCommand>("behavior/motion_command")
        .cache(1)
        .build()
        .await
        .wrap_err("failed to build motion_command subscriber")?;
    let walking_velocity_limits_cache = node
        .subscriber::<WalkingVelocityLimits>(WALKING_VELOCITY_LIMITS_TOPIC)
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await
        .wrap_err("failed to build walking_velocity_limits subscriber")?;

    let serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")
        .cache(1)
        .build()
        .await?;

    let motion_emergency_stop_pub = node
        .publisher::<()>("motion/emergency_stop")
        .build()
        .await
        .wrap_err("failed to build emergency stop publisher")?;

    let robot_command_pub = node
        .publisher::<RobotCommand>(ROBOT_COMMAND_TOPIC)
        .build()
        .await
        .wrap_err("failed to build robot_command publisher")?;

    let inference_qos = QosProfile {
        reliability: QosReliability::BestEffort,
        ..Default::default()
    };
    let walk_inference_client = node
        .service_client::<WalkInferenceService>(WALK_INFERENCE_SERVICE)
        .qos(inference_qos)
        .build()
        .await
        .wrap_err("failed to build walk inference service client")?;

    let kick_inference_client = node
        .service_client::<KickInferenceService>(KICK_INFERENCE_SERVICE)
        .qos(inference_qos)
        .build()
        .await
        .wrap_err("failed to build kick inference service client")?;

    let get_up_inference_client = node
        .service_client::<GetUpInferenceService>(GETUP_INFERENCE_SERVICE)
        .qos(inference_qos)
        .build()
        .await
        .wrap_err("failed to build get up inference service client")?;

    let head_motion_client = node
        .service_client::<HeadMotionService>(HEAD_MOTION_SERVICE_TOPIC)
        .build()
        .await
        .wrap_err("failed to build head motion service client")?;

    let joint_limits_sub = node
        .subscriber::<JointLimits>("joint_limits")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let joint_limits = joint_limits_sub
        .recv()
        .await
        .wrap_err("failed to receive joint limits")?;

    joint_limits.validate().map_err(|reason| eyre!(reason))?;

    let clock = node.clock();

    let mut motion_state = MotionState {
        head_motion_client,
        walk_inference_client,
        kick_inference_client,
        get_up_inference_client,
        motion_emergency_stop_pub,
        reset_inference: true,

        // TODO probably bad defaults
        last_joints_command: Joints::fill(MotorCommand::damping()),
        last_timestamp: clock.now(),
    };

    let mut timer = node.create_timer(Duration::from_millis(2));
    let mut cycles_since_last_inference = 0u8;
    let mut walking_velocity_limits = WalkingVelocityLimits::default();

    loop {
        timer.tick().await;
        let now = clock.now();

        let parameters = &parameters.snapshot().typed;
        if let Some(limits) = walking_velocity_limits_cache.get_latest()
            && limits.validate().is_ok()
        {
            walking_velocity_limits = *limits;
        }

        let motion_command = match motion_command_cache.get_latest_with_stamp() {
            Some((timestamp, motion_command)) => {
                let age = now.duration_since(timestamp);

                if age > parameters.maximum_command_age {
                    error!(
                        "motion command is too old, falling back to damping. command age: {} ms, maximum: {} ms",
                        age.as_millis(),
                        parameters.maximum_command_age.as_millis()
                    );

                    motion_state.send_emergency_stop_signal().await?;

                    Arc::new(MotionCommand::Damping)
                } else {
                    motion_command
                }
            }
            None => {
                warn!("behavior did not provide a motion command (yet)!");

                Arc::new(MotionCommand::Damping)
            }
        };

        let motion_plan = MotionPlan::from_motion_command(
            &motion_command,
            &parameters.walking,
            walking_velocity_limits,
        );

        cycles_since_last_inference += 1;
        let do_inference = cycles_since_last_inference >= 10;
        if do_inference {
            cycles_since_last_inference = 0;
        }

        let joint_state = serial_motor_states_sub
            .get_latest()
            .map(|joints_state| {
                joints_state
                    .upper_body_as_ref()
                    .map(|motor_state| motor_state.position)
            })
            .unwrap_or_else(|| {
                warn!("no serial motor state available, using a fallback. Arms might look wonky.");

                UpperBodyJoints::default()
            });

        let robot_command = motion_state
            .infer(
                motion_plan,
                clock,
                parameters,
                &joint_limits,
                joint_state,
                do_inference,
            )
            .await?;

        robot_command_pub.publish(&robot_command).await?;
    }
}

struct MotionState {
    head_motion_client: ServiceClient<HeadMotionService>,
    walk_inference_client: ServiceClient<WalkInferenceService>,
    kick_inference_client: ServiceClient<KickInferenceService>,
    get_up_inference_client: ServiceClient<GetUpInferenceService>,
    motion_emergency_stop_pub: Publisher<()>,
    reset_inference: bool,
    last_joints_command: Joints<MotorCommand>,
    last_timestamp: Time,
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
        walking_velocity_limits: WalkingVelocityLimits,
    ) -> Self {
        match motion_command {
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
                        direction: *kick_direction,
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
                );

                Self::Walk {
                    head_motion: *head,
                    command: limited_walk_command(
                        vector![step.forward, step.left],
                        step.turn,
                        walking_velocity_limits,
                    ),
                }
            }
            MotionCommand::WalkWithVelocity {
                head,
                velocity,
                angular_velocity,
            } => Self::Walk {
                head_motion: *head,
                command: limited_walk_command(
                    *velocity,
                    *angular_velocity,
                    walking_velocity_limits,
                ),
            },
        }
    }
}

fn limited_walk_command(
    velocity: Vector2<Ground>,
    angular_velocity: f32,
    limits: WalkingVelocityLimits,
) -> WalkCommand {
    let (velocity, angular_velocity) = limits.clamp_command(velocity, angular_velocity);
    WalkCommand {
        velocity,
        angular_velocity,
    }
}

impl MotionState {
    async fn infer(
        &mut self,
        motion_plan: MotionPlan,
        clock: &Clock,
        parameters: &Parameters,
        joint_limits: &JointLimits,
        current_arms: UpperBodyJoints<f32>,
        do_inference: bool,
    ) -> Result<RobotCommand> {
        let now = clock.now();

        let robot_command = match motion_plan {
            MotionPlan::Damping => RobotCommand::Damping,
            MotionPlan::Prepare => RobotCommand::Prepare,
            MotionPlan::GetUp { command } => {
                self.infer_get_up(command, parameters, do_inference).await?
            }
            MotionPlan::Walk {
                head_motion,
                command,
            } => {
                self.infer_generic::<WalkInferenceService, _>(
                    head_motion,
                    InferenceRequest::new(
                        command,
                        self.reset_inference,
                        parameters.inference_timeout,
                    ),
                    clock,
                    parameters,
                    joint_limits,
                    current_arms,
                    do_inference,
                )
                .await?
            }
            MotionPlan::Kick {
                head_motion,
                command,
            } => {
                self.infer_generic::<KickInferenceService, _>(
                    head_motion,
                    InferenceRequest::new(
                        command,
                        self.reset_inference,
                        parameters.inference_timeout,
                    ),
                    clock,
                    parameters,
                    joint_limits,
                    current_arms,
                    do_inference,
                )
                .await?
            }
        };

        let robot_command = match robot_command.clamp(joint_limits) {
            Ok(robot_command) => robot_command,
            Err(error) => {
                error!("Invalid final robot command, sending RobotCommand::Damping: {error:#}");

                self.send_emergency_stop_signal().await?;

                RobotCommand::Damping
            }
        };

        if !matches!(robot_command, RobotCommand::Custom { .. }) {
            self.reset_inference = true;
        } else if do_inference {
            // Cached 500 Hz commands must not consume a reset intended for the next inference.
            self.reset_inference = false;
        }
        if let RobotCommand::Custom { joints_command } = &robot_command {
            self.last_joints_command = joints_command.clone();
            self.last_timestamp = now;
        }

        Ok(robot_command)
    }

    #[allow(clippy::too_many_arguments)]
    async fn infer_generic<S: Service<Response = Result<LowerBodyJoints<MotorCommand>, E>>, E>(
        &mut self,
        head_motion: HeadMotion,
        command: S::Request,
        clock: &Clock,
        parameters: &Parameters,
        joint_limits: &JointLimits,
        current_arms: UpperBodyJoints<f32>,
        do_inference: bool,
    ) -> Result<RobotCommand>
    where
        MotionState: InferGeneric<S>,
        E: Message + for<'a> Deserialize<'a> + Serialize + Display,
    {
        let (inference_result, head_motion_result) = if do_inference {
            let inference_fut = InferGeneric::<S>::inference_client(self)
                .call_with_timeout_async(&command, parameters.inference_timeout);
            let head_motion_fut = self
                .head_motion_client
                .call_with_timeout_async(&head_motion, parameters.head_motion_timeout);
            tokio::join!(inference_fut, head_motion_fut)
        } else {
            let head_motion_fut = self
                .head_motion_client
                .call_with_timeout_async(&head_motion, parameters.head_motion_timeout);
            (
                Ok(Ok(self
                    .last_joints_command
                    .lower_body_as_ref()
                    .map(Clone::clone))),
                head_motion_fut.await,
            )
        };
        let head = match head_motion_result {
            Ok(Ok(head_joints)) => head_joints,
            Ok(Err(head_motion_error)) => {
                error!("Head motion failed, damping head joints: {head_motion_error}");

                HeadJoints::fill(MotorCommand::damping())
            }
            Err(ros_z_error) => {
                error!("Failed to call HeadMotion service, damping head motion! {ros_z_error}");

                HeadJoints::fill(MotorCommand::damping())
            }
        };
        let lower_body_command = match inference_result {
            Ok(Ok(joints_command)) => LowerRobotCommand::Custom {
                lower_body_joints_command: joints_command,
            },
            Ok(Err(inference_error)) => {
                error!(
                    "{} inference failed, sending RobotCommand::Damping: {inference_error}",
                    <Self as InferGeneric::<S>>::INFERENCE_NAME
                );

                self.send_emergency_stop_signal().await?;

                LowerRobotCommand::Damping
            }
            Err(ros_z_error) => {
                error!(
                    "Failed to call {} inference service, sending RobotCommand::Damping! {ros_z_error}",
                    <Self as InferGeneric::<S>>::INFERENCE_NAME
                );

                self.send_emergency_stop_signal().await?;

                LowerRobotCommand::Damping
            }
        };
        Ok(match lower_body_command {
            LowerRobotCommand::Custom {
                lower_body_joints_command,
            } => {
                let arms = self.generate_walking_arm_joints(
                    current_arms,
                    &lower_body_joints_command,
                    clock,
                    &parameters.arms,
                    joint_limits,
                );

                let body = BodyJoints::from_lower_and_upper(lower_body_joints_command, arms);

                RobotCommand::Custom {
                    joints_command: Joints::from_head_and_body(head, body),
                }
            }
            LowerRobotCommand::Damping => RobotCommand::Damping,
        })
    }

    async fn infer_get_up(
        &mut self,
        command: GetUpCommand,
        parameters: &Parameters,
        do_inference: bool,
    ) -> Result<RobotCommand> {
        if !do_inference {
            return Ok(RobotCommand::Custom {
                joints_command: self.last_joints_command.clone(),
            });
        }

        let command =
            InferenceRequest::new(command, self.reset_inference, parameters.inference_timeout);
        let inference_result = self
            .get_up_inference_client
            .call_with_timeout_async(&command, parameters.inference_timeout)
            .await;
        Ok(match inference_result {
            Ok(Ok(joints_command)) => RobotCommand::Custom { joints_command },
            Ok(Err(inference_error)) => {
                error!("GetUp Inference failed, sending RobotCommand::Damping: {inference_error}");

                self.send_emergency_stop_signal().await?;

                RobotCommand::Damping
            }
            Err(ros_z_error) => {
                error!(
                    "Failed to call GetUp inference service, sending RobotCommand::Damping! {ros_z_error}"
                );

                RobotCommand::Damping
            }
        })
    }

    fn generate_walking_arm_joints(
        &mut self,
        current_arms: UpperBodyJoints<f32>,
        lower_body_joints_command: &LowerBodyJoints<MotorCommand>,
        clock: &Clock,
        parameters: &ArmParameters,
        joint_limits: &JointLimits,
    ) -> UpperBodyJoints<MotorCommand> {
        let arms_result = self.try_generate_walking_arm_joints(
            current_arms,
            lower_body_joints_command,
            clock,
            parameters,
            joint_limits,
        );

        match arms_result {
            Ok(arms) => arms,
            Err(error) => {
                error!("Failed to generate arm joints, using fallback joints: {error}");

                UpperBodyJoints::fill(MotorCommand::damping())
            }
        }
    }

    fn try_generate_walking_arm_joints(
        &self,
        current_arms: UpperBodyJoints<f32>,
        legs: &LowerBodyJoints<MotorCommand>,
        clock: &Clock,
        parameters: &ArmParameters,
        joint_limits: &JointLimits,
    ) -> Result<UpperBodyJoints<MotorCommand>> {
        let elapsed = clock.now().duration_since(self.last_timestamp);

        let legs = legs
            .map_ref(|motor_command| motor_command.position)
            .clamp(BodyJoints::from(joint_limits.position).into());

        let ratio =
            (elapsed.as_secs_f32() / parameters.arm_blend_duration.as_secs_f32()).clamp(0.0, 1.0);
        let mut target = Joints::fill(0.0);
        for (left, arm, leg_angles, initial, sign) in [
            (
                true,
                &mut target.left_arm,
                &legs.left_leg,
                current_arms.left_arm,
                1.0,
            ),
            (
                false,
                &mut target.right_arm,
                &legs.right_leg,
                current_arms.right_arm,
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

    async fn send_emergency_stop_signal(&self) -> Result<()> {
        self.motion_emergency_stop_pub.publish(&()).await?;

        Ok(())
    }
}

trait InferGeneric<S: Service> {
    const INFERENCE_NAME: &'static str;

    fn inference_client(&self) -> &ServiceClient<S>;
}

impl InferGeneric<WalkInferenceService> for MotionState {
    const INFERENCE_NAME: &'static str = "walk";

    fn inference_client(&self) -> &ServiceClient<WalkInferenceService> {
        &self.walk_inference_client
    }
}

impl InferGeneric<KickInferenceService> for MotionState {
    const INFERENCE_NAME: &'static str = "kick";
    fn inference_client(&self) -> &ServiceClient<KickInferenceService> {
        &self.kick_inference_client
    }
}

#[allow(clippy::large_enum_variant)]
enum LowerRobotCommand {
    Custom {
        lower_body_joints_command: LowerBodyJoints<MotorCommand>,
    },
    Damping,
}
