//! Temporary simulator coordinator: zero-velocity walking with UI-controlled head motion.
use std::{pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use head_motion::node::{HEAD_MOTION_SERVICE_TOPIC, HeadMotionService};
use motion_inference::{
    inference::WalkCommand,
    node::{WALK_INFERENCE_SERVICE, WalkInferenceService},
};
use ros_z::{prelude::*, time::Time};
use types::{
    motion_command::{HeadMotion, MotionCommand},
    robot_command::{JointsCommand, MotionCommand as RobotCommand, MotionType, MotorCommand},
};

const PERIOD: Duration = Duration::from_millis(20);

pub fn run_boxed(context: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(context))
}

async fn run(context: Arc<Context>) -> Result<()> {
    let node = context.create_node("motion").build().await?;
    let requests = node
        .subscriber::<MotionCommand>("behavior/motion_command")
        .cache(1)
        .build()
        .await?;
    let body = node
        .subscriber::<JointsCommand>("motion_inference/dummy_joints")
        .cache(1)
        .build()
        .await?;
    let head = node
        .service_client::<HeadMotionService>(HEAD_MOTION_SERVICE_TOPIC)
        .build()
        .await?;
    let commands = node
        .publisher::<RobotCommand>("commands/motion_command")
        .build()
        .await?;
    let walking = node
        .service_client::<WalkInferenceService>(WALK_INFERENCE_SERVICE)
        .build()
        .await?;
    let walk_request = WalkCommand {
        velocity: linear_algebra::Vector2::zeros(),
        angular_velocity: 0.0,
    };
    let mut tick = node.create_timer(PERIOD);
    let mut last_warning: Option<Time> = None;
    let mut last_walk_warning: Option<Time> = None;
    loop {
        tick.tick().await;
        let Some(body) = body.get_latest() else {
            continue;
        };
        let request = requests
            .get_latest()
            .and_then(|request| request.head_motion())
            .unwrap_or(HeadMotion::Damping);
        let mut joints_command = (*body).clone();
        let (head_result, walk_result) = tokio::join!(
            head.call_with_timeout_async(&request, PERIOD),
            walking.call_with_timeout_async(&walk_request, PERIOD),
        );
        match walk_result
            .map_err(color_eyre::Report::new)
            .and_then(|result| result.map_err(color_eyre::Report::new))
        {
            Ok(legs) => {
                // Keep the ten head/arm entries, and replace all twelve serial leg commands.
                joints_command = joints_command
                    .into_iter()
                    .take(10)
                    .chain(
                        legs.left_leg
                            .into_iter()
                            .chain(legs.right_leg)
                            .map(|motor| MotorCommand {
                                position: motor.position,
                                velocity: motor.velocity,
                                torque: motor.torque,
                                kp: motor.kp,
                                kd: motor.kd,
                            }),
                    )
                    .collect();
            }
            Err(error) => {
                let now = node.clock().now();
                if last_walk_warning
                    .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1))
                {
                    tracing::warn!(%error, "walking inference unavailable; using dummy zero pose");
                    last_walk_warning = Some(now);
                }
            }
        }
        match head_result {
            Ok(head) => joints_command.head = head,
            Err(error) => {
                // Keep body commands running, but never reuse an active head
                // target after a failed service call. Damping uses the dummy's kd.
                for motor in [&mut joints_command.head.yaw, &mut joints_command.head.pitch] {
                    motor.kp = 0.0;
                    motor.velocity = 0.0;
                    motor.torque = 0.0;
                }
                let now = node.clock().now();
                if last_warning
                    .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1))
                {
                    tracing::warn!(?request, %error, "head motion failed; damping head");
                    last_warning = Some(now);
                }
            }
        }
        commands
            .publish(&RobotCommand {
                // Custom mode is needed for the simulator's joint commands, even
                // when the behavior command requests damping or preparation.
                motion_type: MotionType::Walk,
                joints_command,
            })
            .await?;
    }
}
