use color_eyre::eyre::Context;
use color_eyre::Result;
use ros2_client::{ros2::QosPolicies, rustdds::Topic, MessageTypeName, Name, Node};

pub fn create_topic(
    node: &mut Node,
    namespace: &str,
    topic_name: &str,
    type_name: MessageTypeName,
    qos_policy: &QosPolicies,
) -> Result<Topic> {
    node.create_topic(
        &Name::new(namespace, topic_name).wrap_err("failed to create ROS topic name")?,
        type_name,
        qos_policy,
    )
    .wrap_err("failed to create ROS topic")
}
