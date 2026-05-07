use std::{future::pending, ops::Range, sync::Arc};

use color_eyre::Result;
use coordinate_systems::Pixel;
use geometry::line_segment::LineSegment;
use projection::camera_matrix::CameraMatrix;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    filtered_segments::FilteredSegments, image_segments::GenericSegment, line_data::LineData,
    ycbcr422_image::YCbCr422Image,
};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub use_horizontal_segments: bool,
    pub use_vertical_segments: bool,
    pub allowed_line_length_in_field: Range<f32>,
    pub check_edge_types: bool,
    pub check_edge_gradient: bool,
    pub check_line_distance: bool,
    pub check_line_length: bool,
    pub check_line_segments_projection: bool,
    pub gradient_alignment: f32,
    pub gradient_sobel_stride: u32,
    pub margin_for_point_inclusion: f32,
    pub maximum_distance_to_robot: f32,
    pub maximum_fit_distance_in_ground: f32,
    pub maximum_gap_on_line: f32,
    pub maximum_merge_gap_in_pixels: u16,
    pub maximum_number_of_lines: usize,
    pub allowed_projected_segment_length: Range<f32>,
    pub minimum_number_of_points_on_line: usize,
    pub ransac_iterations: usize,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("line_detection")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("line_detection")
        .into_eyre()?;
    let _camera_matrix_sub = node
        .subscriber::<CameraMatrix>("camera_matrix")
        .build()
        .await
        .into_eyre()?;
    let _filtered_segments_sub = node
        .subscriber::<FilteredSegments>("filtered_segments")
        .build()
        .await
        .into_eyre()?;
    let _image_sub = node
        .subscriber::<YCbCr422Image>("ycbcr422_image")
        .build()
        .await
        .into_eyre()?;
    let _lines_in_image_pub = node
        .publisher::<Vec<LineSegment<Pixel>>>("lines_in_image")
        .build()
        .await
        .into_eyre()?;
    // TODO: restructure type layout here, do not use blank tuples
    // let _discarded_lines_pub = node
    //     .publisher::<Vec<(LineSegment<Pixel>, LineDiscardReason)>>("discarded_lines")
    //     .build()
    //     .await
    //     .into_eyre()?;
    let _filtered_segments_output_pub = node
        .publisher::<Vec<GenericSegment>>("line_detection/filtered_segments")
        .build()
        .await
        .into_eyre()?;
    let _line_data_pub = node
        .publisher::<LineData>("line_data")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
