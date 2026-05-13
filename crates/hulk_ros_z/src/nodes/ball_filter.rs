use std::{future::pending, sync::Arc};

use ball_filter::{BallFilter as BallFiltering, BallHypothesis};
use booster::Odometer;
use color_eyre::Result;
use coordinate_systems::{Ground, Pixel};
use geometry::circle::Circle;
use projection::camera_matrix::CameraMatrix;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    ball_detection::BallPercept,
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    parameters::BallFilterParameters,
};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub field_dimensions: FieldDimensions,
    pub ball_filter_configuration: BallFilterParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("ball_filter").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("ball_filter")
        .into_eyre()?;
    let _historic_camera_matrix_sub = node
        .subscriber::<CameraMatrix>("camera_matrix")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _integrated_odometry_sub = node
        .subscriber::<Odometer>("odometer")
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
    let _filter_state_pub = node
        .publisher::<BallFiltering>("ball_filter_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _best_ball_hypothesis_pub = node
        .publisher::<BallHypothesis>("best_ball_hypothesis")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _filtered_balls_in_image_pub = node
        .publisher::<Vec<Circle<Pixel>>>("filtered_balls_in_image")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ball_percepts_pub = node
        .publisher::<Vec<BallPercept>>("ball_percepts")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ball_position_pub = node
        .publisher::<BallPosition<Ground>>("ball_position")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _hypothetical_ball_positions_pub = node
        .publisher::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
