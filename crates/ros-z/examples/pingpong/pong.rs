use ros_z::{Result, context::ContextBuilder};
use ros_z_msgs::std_msgs::ByteMultiArray;

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default()
        .with_logging_enabled()
        .build()
        .await?;
    let node = context.create_node("pong_node").build().await?;
    let subscriber = node.subscriber::<ByteMultiArray>("ping").build().await?;
    let publisher = node.publisher::<ByteMultiArray>("pong").build().await?;

    println!("Pong is echoing messages from /ping to /pong...");
    loop {
        let message = subscriber.recv().await?;
        publisher.publish(&message).await?;
    }
}
