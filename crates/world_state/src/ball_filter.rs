use color_eyre::Result;
use linear_algebra::Vector2;
use serde::{Deserialize, Serialize};

use ball_filter::BallFilter as BallFiltering;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::{MainOutput, PerceptionInput};
use types::{
    ball_detection::BallPercept,
    ball_position::{BallPosition, HypotheticalBallPosition},
    cycle_time::CycleTime,
};

#[derive(Deserialize, Serialize)]
pub struct BallFilter {
    ball_filter: BallFiltering,
    last_ball_position: Option<BallPosition<Ground>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    ball_percepts_from_vision: PerceptionInput<Option<Vec<BallPercept>>, "Vision", "balls?">,
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
            last_ball_position: None,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let filtered_ball = context
            .ball_percepts_from_vision
            .persistent
            .last_entry()
            .and_then(|entry| entry.get().last().cloned()??.last());

        let filtered_ball = match (filtered_ball, self.last_ball_position) {
            (Some(ball_percepts), _) => Some(BallPosition {
                position: ball_percepts.percept_in_ground.mean.into(),
                velocity: Vector2::zeros(),
                last_seen: context.cycle_time.start_time,
            }),
            (None, Some(last_ball_position)) => Some(last_ball_position),
            _ => None,
        };

        Ok(MainOutputs {
            ball_position: filtered_ball.into(),
            hypothetical_ball_positions: Vec::new().into(),
        })
    }
}
