use std::{future::pending, sync::Arc};

use color_eyre::Result;
use coordinate_systems::Ground;
use linear_algebra::Framed;
use projection::camera_matrix::CameraMatrix;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    field_color::FieldColorParameters, image_segments::ImageSegments,
    parameters::MedianModeParameters, ycbcr422_image::YCbCr422Image,
};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub horizontal_stride: usize,
    pub vertical_stride_in_ground: Framed<Ground, f32>,
    pub horizontal_edge_threshold: u8,
    pub horizontal_median_mode: MedianModeParameters,
    pub vertical_stride: usize,
    pub vertical_edge_threshold: u8,
    pub vertical_median_mode: MedianModeParameters,
    pub field_color: FieldColorParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("image_segmenter")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("image_segmenter")
        .into_eyre()?;
    let _image_sub = node
        .subscriber::<YCbCr422Image>("ycbcr422_image")
        .build()
        .await
        .into_eyre()?;
    let _camera_matrix_sub = node
        .subscriber::<CameraMatrix>("camera_matrix")
        .build()
        .await
        .into_eyre()?;
    let _image_segments_pub = node
        .publisher::<ImageSegments>("image_segments")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
