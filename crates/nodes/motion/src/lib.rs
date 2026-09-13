use std::{pin::Pin, sync::Arc, time::Duration};

use color_eyre::{Result, eyre::WrapErr};
use linear_algebra::{Point2, Vector2};
use tracing::warn;

use ros_z::{context::Context, node::Node};
use types::{
    motion_command::{HeadMotion, KickPower, MotionCommand},
    robot_command::JointsCommand,
    time_wrapper::TimeWrapper,
};

use crate::inference_service::{InferenceRequest, InferenceService, KickRequest};

mod inference_service;

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

    let inference_client = node
        .service_client::<InferenceService>("services/motion_inference")
        .build()
        .await
        .wrap_err("failed to build inference service client")?;

    let mut tick = node.create_timer(Duration::from_millis(20));

    loop {
        tick.tick().await;

        let action_request = motion_command_cache.get_latest().unwrap_or_else(|| {
            warn!("behavior did not provide a motion command (yet)!");

            Arc::new(MotionCommand::Damping)
        });

        let (head_motion, inference_request) = inference_request_from_action(&action_request);
        if let Some(inference_request) = inference_request {
            let inference_result = inference_client.call_async(&inference_request).await;

            let Ok(inference_result) = inference_result else {
                continue;
            };
        } else {
            todo!();
        }
    }
}

fn inference_request_from_action(
    action_request: &MotionCommand,
) -> (Option<HeadMotion>, Option<InferenceRequest>) {
    match action_request {
        MotionCommand::Damping | MotionCommand::Prepare => (None, None),
        MotionCommand::Stand { head } => (Some(*head), Some(InferenceRequest::Stand)),
        MotionCommand::StandUp => (None, Some(InferenceRequest::GetUp { fast: false })),
        MotionCommand::VisualKick {
            head,
            ball_position,
            kick_direction,
            target_position,
            robot_theta_to_field,
            kick_power,
        } => (
            Some(*head),
            Some(InferenceRequest::Kick {
                soft: false, // TODO
                request: KickRequest {
                    ball_position: *ball_position,
                    ball_velocity: Vector2::zeros(),   // TODO
                    direction: kick_direction.angle(), // TODO verify
                    target_speed: 1.0,                 // TODO
                    strong: match kick_power {
                        KickPower::Rumpelstilzchen => false,
                        KickPower::Schlong => true,
                    },
                    quick: false, // TODO
                },
            }),
        ),
        MotionCommand::Walk {
            head,
            path,
            orientation_mode,
            target_orientation,
            distance_to_be_aligned,
            speed,
        } => todo!(),
        MotionCommand::WalkWithVelocity {
            head,
            velocity,
            angular_velocity,
        } => (
            Some(*head),
            Some(InferenceRequest::Walk {
                velocity: *velocity,
                angular_velocity: *angular_velocity,
            }),
        ),
    }
}
