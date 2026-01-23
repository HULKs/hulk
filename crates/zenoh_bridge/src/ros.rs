use color_eyre::eyre::Context;
use color_eyre::Result;
use ros2_client::{MessageTypeName, Name, Node, Publisher, Subscription};
use serde::Serialize;

pub trait RosNode {
    fn subscribe<T: 'static>(
        &mut self,
        namespace: &'static str,
        topic_name: &'static str,
        type_name: MessageTypeName,
    ) -> Result<Subscription<T>>;

    fn publisher<T: Serialize>(
        &mut self,
        namespace: &'static str,
        topic_name: &'static str,
        type_name: MessageTypeName,
    ) -> Result<Publisher<T>>;
}

impl RosNode for Node {
    fn subscribe<T: 'static>(
        &mut self,
        namespace: &'static str,
        topic_name: &'static str,
        type_name: MessageTypeName,
    ) -> Result<Subscription<T>> {
        let topic = self
            .create_topic(
                &Name::new(namespace, topic_name).wrap_err("failed to create ROS topic name")?,
                type_name,
                &ros2_client::DEFAULT_SUBSCRIPTION_QOS,
            )
            .wrap_err("failed to create ROS topic")?;

        self.create_subscription(&topic, None)
            .wrap_err("failed to create subscription")
    }

    fn publisher<T: Serialize>(
        &mut self,
        namespace: &'static str,
        topic_name: &'static str,
        type_name: MessageTypeName,
    ) -> Result<Publisher<T>> {
        let topic = self
            .create_topic(
                &Name::new(namespace, topic_name).wrap_err("failed to create ROS topic name")?,
                type_name,
                &ros2_client::DEFAULT_SUBSCRIPTION_QOS,
            )
            .wrap_err("failed to create ROS topic")?;

        self.create_publisher(&topic, None)
            .wrap_err("failed to create publisher")
    }
}
