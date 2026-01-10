use booster::{ButtonEventMsg, FallDownState, LowState};
use cdr::{CdrLe, Infinite};
use color_eyre::eyre::{bail, Result, WrapErr};
use futures_util::{pin_mut, select, FutureExt, StreamExt};
use ros2_client::{Context, MessageTypeName, Name, Node, NodeName, NodeOptions, Subscription};
use serde::{de::DeserializeOwned, Serialize};
use zenoh::Session;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let session = zenoh::open(zenoh::Config::default()).await.unwrap();
    let context = Context::new().unwrap();

    let mut node = context
        .new_node(
            NodeName::new("/", "booster_zenoh_bridge").unwrap(),
            NodeOptions::new(),
        )
        .unwrap();

    let button_event_subscription: Subscription<ButtonEventMsg> = subscribe_ros_topic(
        &mut node,
        "button_event",
        MessageTypeName::new("booster_interface", "ButtonEventMsg"),
    );
    let fall_down_state_subscription: Subscription<FallDownState> = subscribe_ros_topic(
        &mut node,
        "fall_down",
        MessageTypeName::new("booster_interface", "FallDownState"),
    );
    let low_state_subscription: Subscription<LowState> = subscribe_ros_topic(
        &mut node,
        "low_state",
        MessageTypeName::new("booster_interface", "LowState"),
    );

    let button_event_forwarder =
        forward_ros_to_zenoh(button_event_subscription, &session, "button_event").fuse();
    let fall_down_state_forwarder =
        forward_ros_to_zenoh(fall_down_state_subscription, &session, "fall_down_state").fuse();
    let low_state_forwarder =
        forward_ros_to_zenoh(low_state_subscription, &session, "low_state").fuse();

    pin_mut!(button_event_forwarder);
    pin_mut!(fall_down_state_forwarder);
    pin_mut!(low_state_forwarder);

    // If no errors occur, none of these futures will complete
    let result = select! {
        result = button_event_forwarder => result,
        result = fall_down_state_forwarder => result,
        result = low_state_forwarder => result,
    };
    result.wrap_err("forwarder error occurred")?;

    unreachable!("forwarder futures can not complete without errors")
}

fn subscribe_ros_topic<T: 'static>(
    node: &mut Node,
    name: &'static str,
    type_name: MessageTypeName,
) -> Subscription<T> {
    let topic = node
        .create_topic(
            &Name::new("/", name).unwrap(),
            type_name,
            &ros2_client::DEFAULT_SUBSCRIPTION_QOS,
        )
        .unwrap();
    // .wrap_err("failed to create ROS topic");

    node.create_subscription(&topic, None).unwrap()
}

async fn forward_ros_to_zenoh<T: 'static + Serialize + DeserializeOwned>(
    ros_subscription: Subscription<T>,
    zenoh_session: &Session,
    name: &'static str,
) -> Result<()> {
    // .wrap_err("failed to create ROS subscriber")?;

    let zenoh_publisher = zenoh_session
        .declare_publisher(format!("booster/{name}"))
        .await
        .unwrap();
    // .wrap_err("failed to create Zenoh publisher")?;

    let stream = ros_subscription.async_stream();
    pin_mut!(stream);

    while let Some(result) = stream.next().await {
        let (message, _) = result.wrap_err("read error occurred")?;
        let serialized_message = cdr::serialize::<_, _, CdrLe>(&message, Infinite)
            .wrap_err("failed to serialize received message")?;
        zenoh_publisher.put(serialized_message).await.unwrap();
    }

    bail!("no more available messages from ROS")
}
