use ros_z::{Result, context::ContextBuilder};
use ros_z_msgs::std_msgs::String as RosString;

#[tokio::main]
async fn main() -> Result<()> {
    zenoh::init_log_from_env_or("error");

    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("listener").build().await?;
    let subscriber = node.subscriber::<RosString>("chatter").build().await?;

    println!("Listening for String messages on /chatter...");
    while let Ok(received) = subscriber.recv_with_metadata().await {
        println!(
            "I heard: {} transport={:?} source={:?}",
            received.data, received.transport_time, received.source_time
        );
    }

    Ok(())
}
