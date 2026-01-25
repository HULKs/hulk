use std::{thread, time};

use color_eyre::eyre::{Result, WrapErr};
use ros2_client::{
    ros2::{
        policy::{Durability, History},
        Duration, QosPolicyBuilder,
    },
    Context, MessageTypeName, Name, NodeName, NodeOptions,
};

use crate::rpc::Request;

mod rpc;

fn main() -> Result<()> {
    env_logger::init();

    let ros_context = Context::new().wrap_err("failed to create ROS context")?;
    let mut ros_node = ros_context
        .new_node(
            NodeName::new("/", "high_level_interface").wrap_err("failed to create node name")?,
            NodeOptions::new(),
        )
        .wrap_err("failed to create ROS node")?;

    let topic = ros_node
        .create_topic(
            &Name::new("/", "LocoApiTopicReq").wrap_err("failed to create ROS topic name")?,
            MessageTypeName::new("booster_msgs", "RpcReqMsg"),
            &QosPolicyBuilder::new()
                .reliable(Duration::INFINITE)
                .history(History::KeepLast { depth: 1 })
                .durability(Durability::TransientLocal)
                .build(),
        )
        .wrap_err("failed to create ROS topic")?;

    let publisher = ros_node
        .create_publisher(&topic, None)
        .wrap_err("failed to create publisher")?;

    thread::sleep(time::Duration::from_millis(500));

    let request = Request::new("{\"api_id\":2000}", "{\"mode\":3}");
    publisher
        .publish(request)
        .wrap_err("failed to publish topic")?;

    println!("Send request");

    thread::sleep(time::Duration::from_secs(5));

    Ok(())
}
