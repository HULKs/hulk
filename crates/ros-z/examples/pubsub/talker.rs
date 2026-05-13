use std::time::Duration;

use ros_z::{Result, context::ContextBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    zenoh::init_log_from_env_or("error");

    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("talker").build().await?;
    let publisher = node.publisher::<String>("chatter")?.build().await?;

    let mut count = 1_u64;
    loop {
        let message = format!("Hello ros-z: {count}");
        println!("Publishing: {}", message);
        publisher.publish(&message).await?;

        count += 1;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
