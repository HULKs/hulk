use booster::{ButtonEventMsg, FallDownState, LowCommand, LowState};
use cdr::{CdrLe, Infinite};
use color_eyre::eyre::{bail, Result, WrapErr};
use futures_util::{pin_mut, select, FutureExt, StreamExt};
use ros2_client::{
    Context, MessageTypeName, Name, Node, NodeName, NodeOptions, Publisher, Subscription,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;
use zenoh::Session;

trait RosNode {
    fn subscribe<T: 'static>(
        &mut self,
        topic_name: &'static str,
        type_name: MessageTypeName,
    ) -> Subscription<T>;

    fn publisher<T: Serialize>(
        &mut self,
        topic_name: &'static str,
        type_name: MessageTypeName,
    ) -> Publisher<T>;
}

impl RosNode for Node {
    fn subscribe<T: 'static>(
        &mut self,
        topic_name: &'static str,
        type_name: MessageTypeName,
    ) -> Subscription<T> {
        let topic = self
            .create_topic(
                &Name::new("/", topic_name).unwrap(),
                type_name,
                &ros2_client::DEFAULT_SUBSCRIPTION_QOS,
            )
            .unwrap();
        // .wrap_err("failed to create ROS topic");

        self.create_subscription(&topic, None).unwrap()
    }

    fn publisher<T: Serialize>(
        &mut self,
        topic_name: &'static str,
        type_name: MessageTypeName,
    ) -> Publisher<T> {
        let topic = self
            .create_topic(
                &Name::new("/", topic_name).unwrap(),
                type_name,
                &ros2_client::DEFAULT_SUBSCRIPTION_QOS,
            )
            .unwrap();
        // .wrap_err("failed to create ROS topic");

        self.create_publisher(&topic, None).unwrap()
    }
}

trait Bridge {
    async fn forward_from_subscriber<T: 'static + Serialize + DeserializeOwned>(
        &self,
        subscription: Subscription<T>,
        topic_name: &'static str,
    ) -> Result<()>;

    async fn forward_to_publisher<'a, T: Debug + Serialize + Deserialize<'a>>(
        &self,
        publisher: Publisher<T>,
        topic_name: &'static str,
    ) -> Result<()>;
}

impl Bridge for Session {
    async fn forward_from_subscriber<T: 'static + Serialize + DeserializeOwned>(
        &self,
        subscription: Subscription<T>,
        topic_name: &'static str,
    ) -> Result<()> {
        let zenoh_publisher = self
            .declare_publisher(format!("booster/{topic_name}"))
            .await
            .unwrap();
        // .wrap_err("failed to create Zenoh publisher")?;

        let stream = subscription.async_stream();
        pin_mut!(stream);

        while let Some(result) = stream.next().await {
            let (message, _) = result.wrap_err("read error occurred")?;
            let serialized_message = cdr::serialize::<_, _, CdrLe>(&message, Infinite)
                .wrap_err("failed to serialize received message")?;
            zenoh_publisher.put(serialized_message).await.unwrap();
        }

        bail!("no more available messages from ROS")
    }

    async fn forward_to_publisher<'a, T: Debug + Serialize + Deserialize<'a>>(
        &self,
        publisher: Publisher<T>,
        topic_name: &'static str,
    ) -> Result<()> {
        let subscriber = self
            .declare_subscriber(format!("booster/{topic_name}"))
            .await
            .unwrap();

        while let Ok(message) = subscriber.recv_async().await {
            let deserialized_message = cdr::deserialize(&message.payload().to_bytes())
                .wrap_err("deserialization failed")?;
            publisher.publish(deserialized_message).unwrap();
            // .wrap_err("failed to publish message")?;
        }

        bail!("no more available messages from Zenoh")
    }
}

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

    let button_event_subscription: Subscription<ButtonEventMsg> = node.subscribe(
        "button_event",
        MessageTypeName::new("booster_interface", "ButtonEventMsg"),
    );
    let fall_down_state_subscription: Subscription<FallDownState> = node.subscribe(
        "fall_down",
        MessageTypeName::new("booster_interface", "FallDownState"),
    );
    let low_state_subscription: Subscription<LowState> = node.subscribe(
        "low_state",
        MessageTypeName::new("booster_interface", "LowState"),
    );
    let low_command_publisher: Publisher<LowCommand> = node.publisher(
        "joint_ctrl",
        MessageTypeName::new("booster_interface", "LowCmd"),
    );

    let button_event_forwarder = session
        .forward_from_subscriber(button_event_subscription, "button_event")
        .fuse();
    let fall_down_state_forwarder = session
        .forward_from_subscriber(fall_down_state_subscription, "fall_down_state")
        .fuse();
    let low_state_forwarder = session
        .forward_from_subscriber(low_state_subscription, "low_state")
        .fuse();
    let low_command_forwarder = session
        .forward_to_publisher(low_command_publisher, "low_command")
        .fuse();

    pin_mut!(button_event_forwarder);
    pin_mut!(fall_down_state_forwarder);
    pin_mut!(low_state_forwarder);
    pin_mut!(low_command_forwarder);

    // If no errors occur, none of these futures will complete
    let result = select! {
        result = button_event_forwarder => result,
        result = fall_down_state_forwarder => result,
        result = low_state_forwarder => result,
        result = low_command_forwarder => result,
    };
    result.wrap_err("forwarder error occurred")?;

    unreachable!("forwarder futures can not complete without errors")
}
