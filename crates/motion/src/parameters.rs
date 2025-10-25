use std::{ops::Range, time::Duration};

use coordinate_systems::Walk;
use geometry::rectangle::Rectangle;
use linear_algebra::{Vector2, Vector3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    joints::{arm::ArmJoints, leg::LegJoints},
    step::Step,
};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Parameters {
    pub injected_step: Step,
}
