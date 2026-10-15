use ros_z::Message;
use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use geometry::circle::Circle;

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Message)]
pub struct BallPercept {
    pub percept_in_ground: MultivariateNormalDistribution<2>,
    pub image_location: Circle<Pixel>,
}
