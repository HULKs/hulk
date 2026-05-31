use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;
use nalgebra as na;

use booster::FallDownState;
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use projection::camera_matrix::CameraMatrix;
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    obstacle_filter::Hypothesis,
    obstacles::Obstacle,
    parameters::ObstacleFilterParameters,
    primary_state::PrimaryState,
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("obstacle_filter").build().await?;

    let _parameters = node.bind_parameter_as::<ObstacleFilterParameters>("obstacle_filter")?;
    let _field_dimensions_sub = node
        .subscriber::<FieldDimensions>("field_dimensions")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _camera_matrix_sub = node
        .subscriber::<CameraMatrix>("camera_matrix")?
        .build()
        .await?;
    let _network_robot_obstacles_sub = node
        .subscriber::<Vec<Point2<Ground>>>("network_robot_obstacles")?
        .build()
        .await?;
    let _current_odometry_to_last_odometry_sub = node
        .subscriber::<na::Isometry2<f32>>("current_odometry_to_last_odometry")?
        .build()
        .await?;
    let _primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _current_ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")?
        .build()
        .await?;
    let _fall_down_state_sub = node
        .subscriber::<FallDownState>("inputs/fall_down_state")?
        .build()
        .await?;
    let _detected_objects_sub = node
        .subscriber::<Vec<Object<RobocupObjectLabel>>>("detected_objects")?
        .build()
        .await?;
    let _obstacle_filter_hypotheses_pub = node
        .publisher::<Vec<Hypothesis>>("obstacle_filter_hypotheses")?
        .build()
        .await?;
    let _obstacles_pub = node
        .publisher::<Vec<Obstacle>>("obstacles")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
