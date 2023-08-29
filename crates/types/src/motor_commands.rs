use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::joints::Joints;

#[derive(Default, Clone, Serialize, Deserialize, SerializeHierarchy, Debug)]
pub struct MotorCommands {
    pub positions: Joints<f32>,

