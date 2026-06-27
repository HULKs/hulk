use ros_z::{Result, context::ContextBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    zenoh::init_log_from_env_or("error");

    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("listener").build().await?;
    let subscriber = node.subscriber::<String>("chatter").build().await?;

    println!("Listening for String messages on /chatter...");
    while let Ok(received) = subscriber.recv_with_metadata().await {
        println!(
            "I heard: {} transport={:?} source={:?}",
            *received, received.transport_time, received.source_time
        );
    }

    Ok(())
}
