use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use ros_z::prelude::*;
use ros2::sensor_msgs::image::Image;
use types::{
    object_detection::{Object, RobocupObjectLabel, YOLOObjectLabel},
    parameters::HydraParameters,
    pose_detection::Pose,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub parameters: HydraParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("inference").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("inference")?;
    let _image_sub = node.subscriber::<Image>("inputs/image")?.build().await?;
    let _inference_duration_pub = node
        .publisher::<Duration>("inference_duration")?
        .build()
        .await?;
    let _post_processing_duration_pub = node
        .publisher::<Duration>("post_processing_duration")?
        .build()
        .await?;
    let _non_maximum_suppression_duration_pub = node
        .publisher::<Duration>("non_maximum_suppression_duration")?
        .build()
        .await?;
    let _detected_objects_pub = node
        .publisher::<Vec<Object<RobocupObjectLabel>>>("detected_objects")?
        .build()
        .await?;
    let _detected_poses_pub = node
        .publisher::<Vec<Pose<YOLOObjectLabel>>>("detected_poses")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
