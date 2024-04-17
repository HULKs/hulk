use std::f32::consts::FRAC_PI_2;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::Interpolate;

use crate::joints::{arm::ArmJoints, mirror::Mirror};

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub enum ArmCommand {
    #[default]
    Swing,
    Activating {
        influence: f32,
        positions: ArmJoints,
    },
    Active {
        positions: ArmJoints,
    },
}

impl Mirror for ArmCommand {
    fn mirrored(self) -> Self {
        match self {
            ArmCommand::Swing => ArmCommand::Swing,
            ArmCommand::Activating {
                influence,
                positions,
            } => ArmCommand::Activating {
                influence,
                positions: positions.mirrored(),
            },
            ArmCommand::Active { positions } => ArmCommand::Active {
                positions: positions.mirrored(),
            },
        }
    }
}

impl ArmCommand {
    pub fn shoulder_pitch(&self) -> f32 {
        match self {
            ArmCommand::Swing => FRAC_PI_2,
            ArmCommand::Activating {
                influence,
                positions,
            } => f32::lerp(*influence, FRAC_PI_2, positions.shoulder_pitch),
            ArmCommand::Active { positions } => positions.shoulder_pitch,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct ArmCommands {
    pub left_arm: ArmCommand,
    pub right_arm: ArmCommand,
}
