use std::time::Duration;

use ros_z::{Result, context::ContextBuilder};
use ros_z_msgs::std_msgs::String as RosString;

#[tokio::main]
async fn main() -> Result<()> {
    zenoh::init_log_from_env_or("error");

    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("talker").build().await?;
    let publisher = node.publisher::<RosString>("chatter").build().await?;

    let mut count = 1_u64;
    loop {
        let message = RosString {
            data: format!("Hello ros-z: {count}"),
        };
        println!("Publishing: {}", message.data);
        publisher.publish(&message).await?;

        count += 1;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
