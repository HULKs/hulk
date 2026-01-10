use booster::LowState;
use cdr::{CdrLe, Infinite};
use color_eyre::eyre::Result;
use ros2_client::{Context, MessageTypeName, Name, NodeName, NodeOptions};

pub fn setup_logger() -> Result<(), fern::InitError> {
    env_logger::init();
    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    setup_logger()?;

    let session = zenoh::open(zenoh::Config::default()).await.unwrap();
    let context = Context::new().unwrap();

    let mut node = context
        .new_node(
            NodeName::new("/", "booster_zenoh_bridge").unwrap(),
            NodeOptions::new(),
        )
        .unwrap();

    let low_state_topic = node
        .create_topic(
            &Name::new("/", "low_state").unwrap(),
            MessageTypeName::new("booster_interface", "LowState"),
            &ros2_client::DEFAULT_SUBSCRIPTION_QOS,
        )
        .unwrap();
    let low_state_subscription = node
        .create_subscription::<LowState>(&low_state_topic, None)
        .unwrap();

    let low_state_publisher = session
        .declare_publisher("booster/low_state")
        .await
        .unwrap();

    loop {
        let (low_state, _) = low_state_subscription.async_take().await?;
        let low_state = cdr::serialize::<_, _, CdrLe>(&low_state, Infinite)?;

        low_state_publisher.put(low_state).await.unwrap();

        println!("low state forwarded");
    }
}
