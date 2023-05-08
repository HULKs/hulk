use nalgebra::{Point2, UnitComplex};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::{PathSegment, Side};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OrientationMode {
    AlignWithPath,
    Override(UnitComplex<f32>),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub enum MotionCommand {
    ArmsUpSquat,
    FallProtection {
        direction: FallDirection,
    },
    Jump {
        direction: JumpDirection,
    },
    Penalized,
    SitDown {
        head: HeadMotion,
    },
    Stand {
        head: HeadMotion,
        is_energy_saving: bool,
    },
    StandUp {
        facing: Facing,
    },
    #[default]
    Unstiff,
    Walk {
        head: HeadMotion,
        path: Vec<PathSegment>,
        left_arm: ArmMotion,
        right_arm: ArmMotion,
        orientation_mode: OrientationMode,
    },
    InWalkKick {
        head: HeadMotion,
        kick: KickVariant,
        kicking_side: Side,
        strength: f32,
    },
}

impl MotionCommand {
    pub fn head_motion(&self) -> Option<HeadMotion> {
        match self {
            MotionCommand::SitDown { head }
            | MotionCommand::Stand { head, .. }
            | MotionCommand::Walk { head, .. }
            | MotionCommand::InWalkKick { head, .. } => Some(*head),
            MotionCommand::Penalized => Some(HeadMotion::ZeroAngles),
            MotionCommand::Unstiff => Some(HeadMotion::Unstiff),
            MotionCommand::ArmsUpSquat
            | MotionCommand::FallProtection { .. }
            | MotionCommand::Jump { .. }
            | MotionCommand::StandUp { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub enum HeadMotion {
    ZeroAngles,
    Center,
    LookAround,
    SearchForLostBall,
    LookAt { target: Point2<f32> },
    LookLeftAndRightOf { target: Point2<f32> },
    Unstiff,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum ArmMotion {
    Swing,
    PullTight,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum KickDirection {
    Back,
    Front,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum KickVariant {
    Forward,
    Turn,
    Side,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum Facing {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum SitDirection {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum FallDirection {
    Backward,
    Forward,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum JumpDirection {
    Left,
    Right,
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy,
)]
pub enum GlanceDirection {
    #[default]
    LeftOfTarget,
    RightOfTarget,
}
