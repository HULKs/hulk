use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::Step;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct JointOverride {
    pub value: f32,
    pub timepoint: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KickStep {
    pub base_step: Step,
    pub hip_pitch_overrides: Option<Vec<JointOverride>>,
    pub ankle_pitch_overrides: Option<Vec<JointOverride>>,
}

impl KickStep {
    pub fn is_strength_scaling(&self) -> bool {
        self.hip_pitch_overrides.is_some() || self.ankle_pitch_overrides.is_some()
    }
}
