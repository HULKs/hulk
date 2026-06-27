mod status_types;

use std::time::Duration;

use ros_z::{Result, context::ContextBuilder};
use status_types::RobotStatus;

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context
        .create_node("robot_status_publisher")
        .build()
        .await?;
    let publisher = node
        .publisher::<RobotStatus>("robot_status")
        .build()
        .await?;

    loop {
        let message = RobotStatus {
            robot_id: "robot_1".to_string(),
            battery_percentage: 87.5,
            position_x: 1.0,
            position_y: 2.0,
            is_moving: true,
        };
        println!(
            "Publishing {} at ({:.1}, {:.1}) with {:.1}% battery",
            message.robot_id, message.position_x, message.position_y, message.battery_percentage
        );
        publisher.publish(&message).await?;

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
