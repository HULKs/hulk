use std::{collections::BTreeMap, time::SystemTime};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use ball_filter::BallFilter as BallFiltering;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::{MainOutput, PerceptionInput};
use projection::{camera_matrices::CameraMatrices, camera_matrix::CameraMatrix};
use types::{
    ball_detection::BallPercept,
    ball_position::{BallPosition, HypotheticalBallPosition},
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
};

#[derive(Deserialize, Serialize)]
pub struct BallFilter {
    ball_filter: BallFiltering,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrices: Input<Option<CameraMatrices>, "camera_matrices?">,
    cycle_time: Input<CycleTime, "cycle_time">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    ball_percepts_from_vision: PerceptionInput<Option<Vec<BallPercept>>, "VisionBottom", "balls?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ball_position: MainOutput<Option<BallPosition<Ground>>>,
    pub hypothetical_ball_positions: MainOutput<Vec<HypotheticalBallPosition<Ground>>>,
}

impl BallFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            ball_filter: Default::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let filtered_ball = context.ball_percepts_from_vision.persistent.pop_first();

        let percept_vector = match filtered_ball {
            Some((time, _)) => None,
            None => None,
        };

        Ok(MainOutputs {
            ball_position: filtered_ball.into(),
            hypothetical_ball_positions: Vec::new().into(),
        })
    }
}
