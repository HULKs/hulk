use serde::{Deserialize, Serialize};

use coordinate_systems::Ground;
use linear_algebra::{Orientation2, Point2};
use serialize_hierarchy::SerializeHierarchy;

use crate::{camera_position::CameraPosition, planned_path::PathSegment, support_foot::Side};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OrientationMode {
    AlignWithPath,
    Override(Orientation2<Ground>),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub enum MotionCommand {
    ArmsUpSquat,
    FallProtection {
        direction: FallDirection,
    },
    Initial {
        head: HeadMotion,
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
            | MotionCommand::Initial { head }
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
    LookAt {
        target: Point2<Ground>,
        pixel_target: PixelTarget,
        camera: Option<CameraPosition>,
    },
    LookLeftAndRightOf {
        target: Point2<Ground>,
    },
    Unstiff,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub enum PixelTarget {
    Bottom,
    #[default]
    Center,
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
