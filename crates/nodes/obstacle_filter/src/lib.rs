use std::{future::pending, sync::Arc};

use color_eyre::Result;
use nalgebra as na;
use serde::{Deserialize, Serialize};

use booster::FallDownState;
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use projection::camera_matrix::CameraMatrix;
use ros_z::{IntoEyreResultExt, prelude::*};
use types::{
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    obstacle_filter::Hypothesis,
    obstacles::Obstacle,
    parameters::ObstacleFilterParameters,
    primary_state::PrimaryState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub field_dimensions: FieldDimensions,
    pub obstacle_filter_parameters: ObstacleFilterParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("obstacle_filter")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("obstacle_filter")
        .into_eyre()?;
    let _camera_matrix_sub = node
        .subscriber::<CameraMatrix>("camera_matrix")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _network_robot_obstacles_sub = node
        .subscriber::<Vec<Point2<Ground>>>("network_robot_obstacles")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _current_odometry_to_last_odometry_sub = node
        .subscriber::<na::Isometry2<f32>>("current_odometry_to_last_odometry")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _current_ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _fall_down_state_sub = node
        .subscriber::<FallDownState>("inputs/fall_down_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _detected_objects_sub = node
        .subscriber::<Vec<Object<RobocupObjectLabel>>>("detected_objects")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _obstacle_filter_hypotheses_pub = node
        .publisher::<Vec<Hypothesis>>("obstacle_filter_hypotheses")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _obstacles_pub = node
        .publisher::<Vec<Obstacle>>("obstacles")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
