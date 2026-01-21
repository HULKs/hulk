use cdr::{CdrLe, Infinite};
use color_eyre::eyre::{bail, Result, WrapErr};
use futures_util::{pin_mut, StreamExt};
use ros2_client::{Publisher, Subscription};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;
use zenoh::Session;

pub async fn forward_zenoh_to_ros<'a, T: Debug + Serialize + Deserialize<'a>>(
    zenoh_session: Session,
    zenoh_topic_name: &'static str,
    ros_publisher: Publisher<T>,
) -> Result<()> {
    let zenoh_subscriber = zenoh_session
        .declare_subscriber(format!("booster/{zenoh_topic_name}"))
        .await
        .unwrap();

    while let Ok(message) = zenoh_subscriber.recv_async().await {
        let deserialized_message =
            cdr::deserialize(&message.payload().to_bytes()).wrap_err("deserialization failed")?;
        ros_publisher.publish(deserialized_message).unwrap();
        // .wrap_err("failed to publish message")?;
    }

    bail!("no more available messages from Zenoh")
}

pub async fn forward_ros_to_zenoh<T: 'static + Serialize + DeserializeOwned>(
    ros_subscription: Subscription<T>,
    zenoh_session: Session,
    zenoh_topic_name: &'static str,
) -> Result<()> {
    let stream = ros_subscription.async_stream();
    pin_mut!(stream);

    let zenoh_publisher = zenoh_session
        .declare_publisher(format!("booster/{zenoh_topic_name}"))
        .await
        .unwrap();
    // .wrap_err("failed to create Zenoh publisher")?;

    while let Some(result) = stream.next().await {
        let (message, _) = result.wrap_err("read error occurred")?;
        let serialized_message = cdr::serialize::<_, _, CdrLe>(&message, Infinite)
            .wrap_err("failed to serialize received message")?;
        zenoh_publisher.put(serialized_message).await.unwrap();
    }

    bail!("no more available messages from ROS")
}
