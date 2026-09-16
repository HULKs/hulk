use std::{pin::Pin, sync::Arc, time::Duration};

use color_eyre::{
    Result,
    eyre::{WrapErr, ensure},
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use head_motion::node::{HEAD_MOTION_SERVICE_TOPIC, HeadMotionService};
use kinematics::joints::{
    Joints,
    body::{BodyJoints, LowerBodyJoints, UpperBodyJoints},
};
use linear_algebra::{Vector2, vector};
use motion_inference::{
    inference::{GetUpCommand, KickCommand, WalkCommand, joints_are_finite},
    locomotion::{KickRequest, leg},
    node::{
        GETUP_INFERENCE_SERVICE, GetUpInferenceService, KICK_INFERENCE_SERVICE,
        KickInferenceService, WALK_INFERENCE_SERVICE, WalkInferenceService,
    },
};
use ros_z::{
    Message,
    context::Context,
    node::Node,
    parameter::NodeParametersExt,
    qos::{QosDurability, QosProfile},
    service::ServiceClient,
    time::Clock,
};
use types::{
    joint_limits::JointLimits,
    motion_command::{HeadMotion, KickPower, MotionCommand},
    robot_command::{DesiredMode, JointsCommand, MotorCommand},
    time_wrapper::TimeWrapper,
};

use crate::walking::{WalkingParameters, step_from_walk_command};

pub mod walking;

pub const MOTION_COMMAND_TOPIC: &str = "commands/motion_command";

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

    let motion_command_pub = node
        .publisher::<types::robot_command::MotionCommand>(MOTION_COMMAND_TOPIC)
        .build()
        .await
        .wrap_err("failed to build joints_command publisher")?;

    let walk_inference_client = node
        .service_client::<WalkInferenceService>(WALK_INFERENCE_SERVICE)
        .build()
        .await
        .wrap_err("failed to build walk inference service client")?;

    let kick_inference_client = node
        .service_client::<KickInferenceService>(KICK_INFERENCE_SERVICE)
        .build()
        .await
        .wrap_err("failed to build kick inference service client")?;

    let get_up_inference_client = node
        .service_client::<GetUpInferenceService>(GETUP_INFERENCE_SERVICE)
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

    let clock = node.clock();

    let mut motion_state = MotionState {
        head_motion_client,
        walk_inference_client,
        kick_inference_client,
        get_up_inference_client,

        // TODO probably bad defaults
        last_arms: TimeWrapper {
            time: clock.now(),
            inner: UpperBodyJoints::fill(0.0),
        },
    };

    let mut timer = node.create_timer(Duration::from_millis(20));

    loop {
        timer.tick().await;
        let parameters = &parameters.snapshot().typed;

        let action_request = motion_command_cache.get_latest().unwrap_or_else(|| {
            warn!("behavior did not provide a motion command (yet)!");

            Arc::new(MotionCommand::Damping)
        });

        let motion_plan = MotionPlan::from_action_request(&action_request, &parameters.walking);
        let desired_mode = match &motion_plan {
            MotionPlan::Damping => DesiredMode::Damping,
            MotionPlan::Prepare => DesiredMode::Prepare,
            MotionPlan::GetUp { .. } => DesiredMode::Custom,
            MotionPlan::Walk { .. } => DesiredMode::Custom,
            MotionPlan::Kick { .. } => DesiredMode::Custom,
        };

        let joints_command = motion_state
            .infer(motion_plan, clock, parameters, &joint_limits)
            .await;

        let motion_command = types::robot_command::MotionCommand {
            desired_mode,
            joints_command,
        };

        motion_command_pub.publish(&motion_command).await?;
    }
}

struct MotionState {
    head_motion_client: ServiceClient<HeadMotionService>,
    walk_inference_client: ServiceClient<WalkInferenceService>,
    kick_inference_client: ServiceClient<KickInferenceService>,
    get_up_inference_client: ServiceClient<GetUpInferenceService>,
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
    fn from_action_request(action_request: &MotionCommand, parameters: &WalkingParameters) -> Self {
        match action_request {
            MotionCommand::Damping => Self::Damping,
            MotionCommand::Prepare => Self::Prepare,
            MotionCommand::Stand { head } => Self::Walk {
                head_motion: *head,
                command: WalkCommand::stand(),
            },
            MotionCommand::StandUp => Self::GetUp {
                command: GetUpCommand { fast: false }, // TODO behavior should decide this
            },
            MotionCommand::VisualKick {
                head,
                ball_position,
                kick_direction,
                target_position: _,
                robot_theta_to_field: _,
                kick_power,
            } => Self::Kick {
                head_motion: *head,
                command: KickCommand {
                    soft: false, // TODO
                    request: KickRequest {
                        ball_position: *ball_position,
                        ball_velocity: Vector2::zeros(),   // TODO
                        direction: kick_direction.angle(), // TODO verify
                        target_speed: 3.4,                 // TODO
                        strong: match kick_power {
                            KickPower::Rumpelstilzchen => false,
                            KickPower::Schlong => true,
                        },
                        quick: false, // TODO
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
        }
    }
}

impl MotionState {
    async fn infer(
        &mut self,
        motion_plan: MotionPlan,
        clock: &Clock,
        parameters: &Parameters,
        joint_limits: &JointLimits,
    ) -> JointsCommand {
        let now = clock.now();

        let joints_command = match motion_plan {
            MotionPlan::Damping | MotionPlan::Prepare => JointsCommand::fill(MotorCommand::zeros()),
            MotionPlan::GetUp { command } => self
                .get_up_inference_client
                .call_async(&command)
                .await
                .expect("failed to call inference service")
                .unwrap()
                .as_ref()
                .clone(),
            MotionPlan::Walk {
                head_motion,
                command,
            } => {
                let inference_fut = self.walk_inference_client.call_async(&command);
                let head_motion_fut = self.head_motion_client.call_async(&head_motion);

                let (inference_result, head_motion_result) =
                    tokio::join!(inference_fut, head_motion_fut);

                let head = head_motion_result.unwrap();
                let lower_body = inference_result.unwrap().unwrap();
                let arms = self
                    .generate_walking_arm_joints(
                        lower_body.as_ref(),
                        clock,
                        &parameters.arms,
                        joint_limits,
                    )
                    .unwrap();

                let body = BodyJoints::from_lower_and_upper(lower_body.as_ref().clone(), arms);

                Joints::from_head_and_body(head, body)
            }
            MotionPlan::Kick {
                head_motion,
                command,
            } => {
                let inference_fut = self.kick_inference_client.call_async(&command);
                let head_motion_fut = self.head_motion_client.call_async(&head_motion);

                let (inference_result, head_motion_result) =
                    tokio::join!(inference_fut, head_motion_fut);

                let head = head_motion_result.unwrap();
                let lower_body = inference_result.unwrap().unwrap();
                let arms = UpperBodyJoints::fill(MotorCommand::zeros());

                let body = BodyJoints::from_lower_and_upper(lower_body.as_ref().clone(), arms);

                Joints::from_head_and_body(head, body)
            }
        };

        self.last_arms = TimeWrapper {
            time: now,
            inner: joints_command
                .clone()
                .body()
                .upper()
                .map(|motor_command| motor_command.position),
        };

        joints_command
    }

    fn generate_walking_arm_joints(
        &self,
        legs: &LowerBodyJoints<MotorCommand>,
        clock: &Clock,
        parameters: &ArmParameters,
        joint_limits: &JointLimits,
    ) -> Result<UpperBodyJoints<MotorCommand>> {
        let elapsed = clock.now().duration_since(self.last_arms.time);

        let positions = legs
            .into_iter()
            .zip(joint_limits.position)
            .map(|(value, [minimum, maximum])| value.position.clamp(minimum, maximum))
            .collect();

        let ratio =
            (elapsed.as_secs_f32() / parameters.arm_blend_duration.as_secs_f32()).clamp(0.0, 1.0);
        let mut target = Joints::fill(0.0);
        for (left, arm, initial, sign) in [
            (
                true,
                &mut target.left_arm,
                self.last_arms.inner.left_arm,
                1.0,
            ),
            (
                false,
                &mut target.right_arm,
                self.last_arms.inner.right_arm,
                -1.0,
            ),
        ] {
            let (sole, knee) = leg(&positions, left);
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
