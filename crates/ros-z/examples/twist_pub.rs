use std::time::Duration;

use ros_z::{Result, context::ContextBuilder};
use ros_z_msgs::geometry_msgs::{Twist, Vector3};

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("twist_publisher").build().await?;
    let zpub = node.publisher::<Twist>("cmd_vel").build().await?;

    println!("Publishing Twist messages on /cmd_vel...");

    let mut counter = 0.0_f64;
    loop {
        let message = Twist {
            linear: Vector3 {
                x: 0.5 * (counter * 0.1).sin(),
                y: 0.0,
                z: 0.0,
            },
            angular: Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.3 * (counter * 0.1).cos(),
            },
        };

        zpub.publish(&message).await?;
        println!(
            "Published: linear.x={:.2}, angular.z={:.2}",
            message.linear.x, message.angular.z
        );

        counter += 1.0;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
