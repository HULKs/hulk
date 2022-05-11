use macros::SerializeHierarchy;
use nalgebra::{Isometry2, Point2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, SerializeHierarchy, Serialize, Deserialize)]
pub struct MotionCommand {
    #[leaf]
    pub motion: Motion,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum Motion {
    FallProtection {
        direction: FallDirection,
    },
    Jump {
        direction: JumpDirection,
    },
    Kick {
        head: HeadMotion,
        direction: KickDirection,
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
    Unstiff,
    Walk {
        head: HeadMotion,
        in_walk_kick: InWalkKick,
        left_arm: ArmMotion,
        right_arm: ArmMotion,
        target_pose: Isometry2<f32>,
    },
}

impl Default for Motion {
    fn default() -> Self {
        Self::Unstiff
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum HeadMotion {
    ZeroAngles,
    Center,
    LookAround,
    LookAt { target: Point2<f32> },
    Unstiff,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum KickDirection {
    Back,
    Front,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum InWalkKick {
    None,
    Left,
    Right,
    TurnLeft,
    TurnRight,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum ArmMotion {
    PullBack,
    Swing,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum Facing {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum SitDirection {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum FallDirection {
    Backward,
    Forward,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum JumpDirection {
    Left,
    Squat,
    Right,
}
