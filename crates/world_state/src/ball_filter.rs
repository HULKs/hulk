use color_eyre::{eyre::OptionExt, Result};
use linear_algebra::Vector2;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Ground;
use framework::MainOutput;
use types::{
    ball_detection::BallPercept,
    ball_position::{BallPosition, HypotheticalBallPosition},
    cycle_time::CycleTime,
    parameters::BallFilterParameters,
};

#[derive(Deserialize, Serialize)]
pub struct BallFilter {
    last_ball_position: Option<BallPosition<Ground>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    ball_percepts: Input<Option<Vec<BallPercept>>, "balls?">,
    ball_filter_parameter: Parameter<BallFilterParameters, "ball_filter">,
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
            last_ball_position: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let filtered_ball_percept = context.ball_percepts.ok_or_eyre("no ball percept")?.last();

        let ball_position = filtered_ball_percept
            .and_then(|ball_percept| {
                self.last_ball_position = Some(BallPosition {
                    position: ball_percept.percept_in_ground.mean.into(),
                    velocity: Vector2::zeros(),
                    last_seen: context.cycle_time.start_time,
                });
                self.last_ball_position
            })
            .or_else(|| {
                if let Some(last_ball_position) = self.last_ball_position {
                    if context
                        .cycle_time
                        .start_time
                        .duration_since(last_ball_position.last_seen)
                        .expect("time ran backwards")
                        < context.ball_filter_parameter.hypothesis_timeout
                    {
                        return Some(last_ball_position);
                    } else {
                        self.last_ball_position = None;
                    }
                }
                None
            });

        Ok(MainOutputs {
            ball_position: ball_position.into(),
            hypothetical_ball_positions: Vec::new().into(),
        })
    }
}
