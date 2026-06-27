mod status_types;

use ros_z::{Result, context::ContextBuilder};
use status_types::RobotStatus;

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context
        .create_node("robot_status_subscriber")
        .build()
        .await?;
    let subscriber = node
        .subscriber::<RobotStatus>("robot_status")
        .build()
        .await?;

    println!("Listening for RobotStatus messages on /robot_status...");
    loop {
        let message = subscriber.recv().await?;
        println!(
            "{}: pos=({:.1}, {:.1}), battery={:.1}%, moving={}",
            message.robot_id,
            message.position_x,
            message.position_y,
            message.battery_percentage,
            message.is_moving
        );
    }
}
