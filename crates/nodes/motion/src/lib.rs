use std::{pin::Pin, sync::Arc, time::Duration};

use color_eyre::{Result, eyre::WrapErr};
use head_motion::node::{HEAD_MOTION_SERVICE_TOPIC, HeadMotionService};
use linear_algebra::Vector2;
use tracing::warn;

use kinematics::joints::{
    Joints,
    body::{BodyJoints, UpperBodyJoints},
};
use motion_inference::{
    inference::{GetUpCommand, KickCommand, WalkCommand},
    locomotion::KickRequest,
    node::{
        GETUP_INFERENCE_SERVICE, GetUpInferenceService, KICK_INFERENCE_SERVICE,
        KickInferenceService, WALK_INFERENCE_SERVICE, WalkInferenceService,
    },
};
use ros_z::{context::Context, node::Node, service::ServiceClient};
use types::{
    motion_command::{HeadMotion, KickPower, MotionCommand},
    robot_command::{JointsCommand, MotorCommand},
};

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

    let motion_command_cache = node
        .subscriber::<MotionCommand>("behavior/motion_command")
        .cache(1)
        .build()
        .await
        .wrap_err("failed to build motion_command subscriber")?;

    let joints_command_pub = node
        .publisher::<JointsCommand>("commands/joints_command")
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

    let mut timer = node.create_timer(Duration::from_millis(20));

    loop {
        timer.tick().await;

        let action_request = motion_command_cache.get_latest().unwrap_or_else(|| {
            warn!("behavior did not provide a motion command (yet)!");

            Arc::new(MotionCommand::Damping)
        });

        let motion_plan = MotionPlan::from_action_request(&action_request);

        let joints_command = motion_plan
            .infer(
                &head_motion_client,
                &walk_inference_client,
                &kick_inference_client,
                &get_up_inference_client,
            )
            .await;

        joints_command_pub.publish(&joints_command).await?;
    }
}

enum MotionPlan {
    DoNothing,
    GetUp {
        command: GetUpCommand,
    },
    // Stand {
    //     head_motion: HeadMotion,
    //     inference_request: InferenceRequest,
    // },
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
    fn from_action_request(action_request: &MotionCommand) -> Self {
        match action_request {
            MotionCommand::Damping | MotionCommand::Prepare => Self::DoNothing,
            MotionCommand::Stand { head } => Self::Walk {
                head_motion: *head,
                command: WalkCommand {
                    velocity: Vector2::zeros(),
                    angular_velocity: 0.0,
                },
            },
            MotionCommand::StandUp => Self::GetUp {
                command: GetUpCommand { fast: false },
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
            MotionCommand::Walk { .. } => todo!(),
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

    async fn infer(
        self,
        head_motion_client: &ServiceClient<HeadMotionService>,
        walk_inference_client: &ServiceClient<WalkInferenceService>,
        kick_inference_client: &ServiceClient<KickInferenceService>,
        get_up_inference_client: &ServiceClient<GetUpInferenceService>,
    ) -> JointsCommand {
        match self {
            MotionPlan::DoNothing => JointsCommand::fill(MotorCommand::zeros()),
            MotionPlan::GetUp { command } => get_up_inference_client
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
                let inference_fut = walk_inference_client.call_async(&command);
                let head_motion_fut = head_motion_client.call_async(&head_motion);

                let (inference_result, head_motion_result) =
                    tokio::join!(inference_fut, head_motion_fut);

                let head = head_motion_result.unwrap();
                let lower_body = inference_result.unwrap().unwrap();
                let arms = UpperBodyJoints::fill(MotorCommand::zeros());

                let body = BodyJoints::from_lower_and_upper(lower_body.as_ref().clone(), arms);

                Joints::from_head_and_body(head, body)
            }
            MotionPlan::Kick {
                head_motion,
                command,
            } => {
                let inference_fut = kick_inference_client.call_async(&command);
                let head_motion_fut = head_motion_client.call_async(&head_motion);

                let (inference_result, head_motion_result) =
                    tokio::join!(inference_fut, head_motion_fut);

                let head = head_motion_result.unwrap();
                let lower_body = inference_result.unwrap().unwrap();
                let arms = UpperBodyJoints::fill(MotorCommand::zeros());

                let body = BodyJoints::from_lower_and_upper(lower_body.as_ref().clone(), arms);

                Joints::from_head_and_body(head, body)
            }
        }
    }
}
