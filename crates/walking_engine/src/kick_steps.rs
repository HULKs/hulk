use std::time::Duration;

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{motion_command::KickVariant, step::Step};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct JointOverride {
    pub value: f32,
    pub timepoint: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JointOverrides {
    pub hip_pitch: Option<Vec<JointOverride>>,
    pub ankle_pitch: Option<Vec<JointOverride>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KickStep {
    pub base_step: Step,
    pub step_duration: Duration,
    pub foot_lift_apex: f32,
    pub midpoint: f32,
    pub support_overrides: JointOverrides,
    pub swing_overrides: JointOverrides,
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct KickSteps {
    pub forward: Vec<KickStep>,
    pub turn: Vec<KickStep>,
    pub side: Vec<KickStep>,
}

impl KickSteps {
    pub fn get_steps(&self, variant: KickVariant) -> &[KickStep] {
        match variant {
            KickVariant::Forward => &self.forward,
            KickVariant::Turn => &self.turn,
            KickVariant::Side => &self.side,
        }
    }

    pub fn num_steps(&self, variant: KickVariant) -> usize {
        self.get_steps(variant).len()
    }

    pub fn get_step_at(&self, variant: KickVariant, index: usize) -> &KickStep {
        &self.get_steps(variant)[index]
    }
}
