//! Temporary head-controller test path with a dummy body pose.
use std::{pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use head_motion::node::{HEAD_MOTION_SERVICE_TOPIC, HeadMotionService};
use ros_z::{prelude::*, time::Time};
use types::{
    motion_command::{HeadMotion, MotionCommand},
    robot_command::{JointsCommand, MotionCommand as RobotCommand, MotionType},
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
    let mut tick = node.create_timer(PERIOD);
    let mut last_warning: Option<Time> = None;
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
        match head.call_with_timeout_async(&request, PERIOD).await {
            Ok(head) => joints_command.head = head,
            Err(error) => {
                // Keep the zero-pose body running, but never reuse an active head
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
                // Custom mode is needed for the simulator's zero-pose body, even
                // when the behavior command requests damping or preparation.
                motion_type: MotionType::Walk,
                joints_command,
            })
            .await?;
    }
}
