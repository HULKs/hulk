use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use ros_z::prelude::*;
use ros2::sensor_msgs::image::Image;
use serde::{Deserialize, Serialize};
use types::{
    object_detection::{Object, RobocupObjectLabel, YOLOObjectLabel},
    parameters::HydraParameters,
    pose_detection::Pose,
};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub parameters: HydraParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("inference").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("inference")
        .into_eyre()?;
    let _image_sub = node
        .subscriber::<Image>("image")
        .build()
        .await
        .into_eyre()?;
    let _inference_duration_pub = node
        .publisher::<Duration>("inference_duration")
        .build()
        .await
        .into_eyre()?;
    let _post_processing_duration_pub = node
        .publisher::<Duration>("post_processing_duration")
        .build()
        .await
        .into_eyre()?;
    let _non_maximum_suppression_duration_pub = node
        .publisher::<Duration>("non_maximum_suppression_duration")
        .build()
        .await
        .into_eyre()?;
    let _detected_objects_pub = node
        .publisher::<Vec<Object<RobocupObjectLabel>>>("detected_objects")
        .build()
        .await
        .into_eyre()?;
    let _detected_poses_pub = node
        .publisher::<Vec<Pose<YOLOObjectLabel>>>("detected_poses")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
