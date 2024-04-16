use serde::{Deserialize, Serialize};

use coordinate_systems::Ground;
use linear_algebra::{Orientation2, Point2};
use serialize_hierarchy::SerializeHierarchy;

use crate::{
    camera_position::CameraPosition,
    fall_state::{Direction, Kind},
    planned_path::PathSegment,
    support_foot::Side,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OrientationMode {
    AlignWithPath,
    Override(Orientation2<Ground>),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub enum MotionCommand {
    ArmsUpSquat,
    FallProtection {
        direction: Direction,
    },
    Initial,
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
        facing: Kind,
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
        left_arm: ArmMotion,
        right_arm: ArmMotion,
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
            MotionCommand::Penalized | MotionCommand::Initial => Some(HeadMotion::ZeroAngles),
            MotionCommand::Unstiff => Some(HeadMotion::Unstiff),
            MotionCommand::ArmsUpSquat
            | MotionCommand::FallProtection { .. }
            | MotionCommand::Jump { .. }
            | MotionCommand::StandUp { .. } => None,
        }
    }

    pub fn arm_motion(&self, side: Side) -> Option<ArmMotion> {
        match self {
            MotionCommand::Walk {
                left_arm,
                right_arm,
                ..
            } => match side {
                Side::Left => Some(*left_arm),
                Side::Right => Some(*right_arm),
            },
            MotionCommand::InWalkKick {
                left_arm,
                right_arm,
                ..
            } => match side {
                Side::Left => Some(*left_arm),
                Side::Right => Some(*right_arm),
            },
            _ => None,
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
        camera: Option<CameraPosition>,
    },
    LookLeftAndRightOf {
        target: Point2<Ground>,
    },
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
pub enum SitDirection {
    Down,
    Up,
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
