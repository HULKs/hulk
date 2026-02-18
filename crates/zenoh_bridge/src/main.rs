mod bridge;
mod error;
mod ros;

use std::fmt::Debug;

use booster::{ButtonEventMsg, FallDownState, LowCommand, LowState};
use color_eyre::eyre::{Result, WrapErr};
use futures_util::{future::Fuse, select, FutureExt};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use ros2_client::{
    Context, MessageTypeName, Node, NodeName, NodeOptions, Publisher, Subscription,
    DEFAULT_PUBLISHER_QOS, DEFAULT_SUBSCRIPTION_QOS,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::task::JoinHandle;
use zenoh::Session;

use crate::{
    bridge::{forward_ros_to_zenoh, forward_zenoh_to_ros},
    error::Error,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let ros_context = Context::new().wrap_err("failed to create ROS context")?;
    let mut ros_node = ros_context
        .new_node(
            NodeName::new("/", "booster_zenoh_bridge").wrap_err("failed to create node name")?,
            NodeOptions::new(),
        )
        .wrap_err("failed to create ROS node")?;

    let zenoh_session = zenoh::open(zenoh::Config::default())
        .await
        .map_err(Error::Zenoh)
        .wrap_err("failed to create Zenoh session")?;

    let mut button_event_forwarder = spawn_ros_to_zenoh_forwarder::<ButtonEventMsg>(
        &mut ros_node,
        zenoh_session.clone(),
        "/",
        "button_event",
        MessageTypeName::new("booster_interface", "ButtonEventMsg"),
        "button_event",
    )?;
    let mut fall_down_state_forwarder = spawn_ros_to_zenoh_forwarder::<FallDownState>(
        &mut ros_node,
        zenoh_session.clone(),
        "/",
        "fall_down",
        MessageTypeName::new("booster_interface", "FallDownState"),
        "fall_down_state",
    )?;
    let mut low_state_forwarder = spawn_ros_to_zenoh_forwarder::<LowState>(
        &mut ros_node,
        zenoh_session.clone(),
        "/",
        "low_state",
        MessageTypeName::new("booster_interface", "LowState"),
        "low_state",
    )?;
    // let mut origin_left_image_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
    //     &mut ros_node,
    //     zenoh_session.clone(),
    //     "/StereoNetNode",
    //     "origin_left_image",
    //     MessageTypeName::new("sensor_msgs", "Image"),
    //     "origin_left_image",
    // )?;
    // let mut origin_right_image_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
    //     &mut ros_node,
    //     zenoh_session.clone(),
    //     "/StereoNetNode",
    //     "origin_right_image",
    //     MessageTypeName::new("sensor_msgs", "Image"),
    //     "origin_right_image",
    // )?;
    let mut rectified_image_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
        &mut ros_node,
        zenoh_session.clone(),
        "/booster_camera_bridge/StereoNetNode",
        "rectified_image",
        MessageTypeName::new("sensor_msgs", "Image"),
        "rectified_image",
    )?;
    let mut rectified_right_image_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
        &mut ros_node,
        zenoh_session.clone(),
        "/booster_camera_bridge/StereoNetNode",
        "rectified_right_image",
        MessageTypeName::new("sensor_msgs", "Image"),
        "rectified_right_image",
    )?;
    let mut stereonet_depth_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
        &mut ros_node,
        zenoh_session.clone(),
        "/booster_camera_bridge/StereoNetNode",
        "stereonet_depth",
        MessageTypeName::new("sensor_msgs", "Image"),
        "stereonet_depth",
    )?;
    // let mut stereonet_depth_camera_info_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
    //     &mut ros_node,
    //     zenoh_session.clone(),
    //     "/StereoNetNode/stereonet_depth",
    //     "camera_info",
    //     MessageTypeName::new("sensor_msgs", "CameraInfo"),
    //     "stereonet_depth/camera_info",
    // )?;
    let mut stereonet_visual_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
        &mut ros_node,
        zenoh_session.clone(),
        "/booster_camera_bridge/StereoNetNode",
        "stereonet_visual",
        MessageTypeName::new("sensor_msgs", "Image"),
        "stereonet_visual",
    )?;
    // let mut image_combine_raw_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
    //     &mut ros_node,
    //     zenoh_session.clone(),
    //     "/",
    //     "image_combine_raw",
    //     MessageTypeName::new("sensor_msgs", "Image"),
    //     "image_combine_raw",
    // )?;
    let mut image_left_raw_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
        &mut ros_node,
        zenoh_session.clone(),
        "/booster_camera_bridge",
        "image_left_raw",
        MessageTypeName::new("sensor_msgs", "Image"),
        "image_left_raw",
    )?;
    let mut image_left_raw_camera_info_forwarder = spawn_ros_to_zenoh_forwarder::<CameraInfo>(
        &mut ros_node,
        zenoh_session.clone(),
        "/booster_camera_bridge/image_left_raw",
        "camera_info",
        MessageTypeName::new("sensor_msgs", "CameraInfo"),
        "image_left_raw/camera_info",
    )?;
    let mut image_right_raw_forwarder = spawn_ros_to_zenoh_forwarder::<Image>(
        &mut ros_node,
        zenoh_session.clone(),
        "/booster_camera_bridge",
        "image_right_raw",
        MessageTypeName::new("sensor_msgs", "Image"),
        "image_right_raw",
    )?;
    let mut image_right_raw_camera_info_forwarder = spawn_ros_to_zenoh_forwarder::<CameraInfo>(
        &mut ros_node,
        zenoh_session.clone(),
        "/booster_camera_bridge/image_right_raw",
        "camera_info",
        MessageTypeName::new("sensor_msgs", "CameraInfo"),
        "image_right_raw/camera_info",
    )?;

    let mut low_command_forwarder = spawn_zenoh_to_ros_forwarder::<LowCommand>(
        &mut ros_node,
        zenoh_session.clone(),
        "/",
        "joint_ctrl",
        MessageTypeName::new("booster_interface", "LowCmd"),
        "joint_ctrl",
    )?;

    // If no errors occur, none of these futures will complete
    let result = select! {
        result = button_event_forwarder => result,
        result = fall_down_state_forwarder => result,
        result = low_state_forwarder => result,
        // result = origin_left_image_forwarder => result,
        // result = origin_right_image_forwarder => result,
        result = rectified_image_forwarder => result,
        result = rectified_right_image_forwarder => result,
        result = stereonet_depth_forwarder => result,
        // result = stereonet_depth_camera_info_forwarder => result,
        result = stereonet_visual_forwarder => result,
        // result = image_combine_raw_forwarder => result,
        result = image_left_raw_forwarder => result,
        result = image_left_raw_camera_info_forwarder => result,
        result = image_right_raw_forwarder => result,
        result = image_right_raw_camera_info_forwarder => result,
        result = low_command_forwarder => result,
    }
    .wrap_err("failed to run forwarder to completion")?;

    result.wrap_err("forwarder error occurred")?;

    unreachable!("forwarder futures can not complete without errors")
}

fn spawn_ros_to_zenoh_forwarder<T: 'static + Serialize + DeserializeOwned + Send + Sync>(
    ros_node: &mut Node,
    zenoh_session: Session,
    ros_namespace: &'static str,
    ros_topic_name: &'static str,
    ros_type_name: MessageTypeName,
    zenoh_topic_name: &'static str,
) -> Result<Fuse<JoinHandle<Result<()>>>> {
    let ros_topic = ros::create_topic(
        ros_node,
        ros_namespace,
        ros_topic_name,
        ros_type_name,
        &DEFAULT_SUBSCRIPTION_QOS,
    )?;
    let ros_subscription: Subscription<T> = ros_node
        .create_subscription(&ros_topic, None)
        .wrap_err("failed to create subscription")?;

    Ok(tokio::spawn(forward_ros_to_zenoh(
        ros_subscription,
        zenoh_session,
        zenoh_topic_name,
    ))
    .fuse())
}

fn spawn_zenoh_to_ros_forwarder<T: 'static + Serialize + DeserializeOwned + Send + Sync + Debug>(
    ros_node: &mut Node,
    zenoh_session: Session,
    ros_namespace: &'static str,
    ros_topic_name: &'static str,
    ros_type_name: MessageTypeName,
    zenoh_topic_name: &'static str,
) -> Result<Fuse<JoinHandle<Result<()>>>> {
    let ros_topic = ros::create_topic(
        ros_node,
        ros_namespace,
        ros_topic_name,
        ros_type_name,
        &DEFAULT_PUBLISHER_QOS,
    )?;
    let ros_publisher: Publisher<T> = ros_node
        .create_publisher(&ros_topic, None)
        .wrap_err("failed to create publisher")?;

    Ok(tokio::spawn(forward_zenoh_to_ros(
        zenoh_session,
        zenoh_topic_name,
        ros_publisher,
    ))
    .fuse())
}
