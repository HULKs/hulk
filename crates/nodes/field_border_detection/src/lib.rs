use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use linear_algebra::Point2;
use projection::camera_matrix::CameraMatrix;
use ros_z::{IntoEyreResultExt, prelude::*};
use types::{field_border::FieldBorder, image_segments::ImageSegments};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub enable: bool,
    pub angle_threshold: f32,
    pub first_line_association_distance: f32,
    pub min_points_per_line: usize,
    pub second_line_association_distance: f32,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("field_border_detection")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("field_border_detection")
        .into_eyre()?;
    let _camera_matrix_sub = node
        .subscriber::<CameraMatrix>("camera_matrix")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _image_segments_sub = node
        .subscriber::<ImageSegments>("image_segments")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _field_border_points_pub = node
        .publisher::<Vec<Point2<Pixel>>>("field_border_points")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _field_border_pub = node
        .publisher::<FieldBorder>("field_border")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
