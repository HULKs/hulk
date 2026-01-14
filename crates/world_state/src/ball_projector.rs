use color_eyre::Result;
use linear_algebra::point;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use geometry::circle::Circle;

use types::{
    ball_detection::BallPercept, multivariate_normal_distribution::MultivariateNormalDistribution,
};

#[derive(Deserialize, Serialize)]
pub struct BallProjector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub balls: MainOutput<Option<Vec<BallPercept>>>,
}

impl BallProjector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        let dummy_vec = vec![BallPercept {
            percept_in_ground: MultivariateNormalDistribution {
                mean: [2.0, 1.0].into(),
                covariance: [[0.0, 0.0], [0.0, 0.0]].into(),
            },
            image_location: Circle {
                center: point![0.0, 0.0],
                radius: 10.0,
            },
        }];

        Ok(MainOutputs {
            balls: Some(dummy_vec).into(),
        })
    }
}
