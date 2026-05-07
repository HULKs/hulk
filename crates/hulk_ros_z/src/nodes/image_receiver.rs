use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use ros2::sensor_msgs::image::Image;
use types::ycbcr422_image::YCbCr422Image;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("image_receiver")
        .build()
        .await
        .into_eyre()?;
    let _image_pub = node.publisher::<Image>("image").build().await.into_eyre()?;
    let _ycbcr422_image_pub = node
        .publisher::<YCbCr422Image>("ycbcr422_image")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
