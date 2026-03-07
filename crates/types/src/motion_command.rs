use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use linear_algebra::{Orientation2, Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{fall_state::FallingDirection, path::Path, support_foot::Side};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
)]
pub enum WalkSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
)]
pub enum OrientationMode {
    Unspecified,
    AlignWithPath,
    LookTowards {
        direction: Orientation2<Ground>,
        tolerance: f32,
    },
    LookAt {
        target: Point2<Ground>,
        tolerance: f32,
    },
}

#[derive(
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
)]
pub enum MotionCommand {
    ArmsUpSquat,
    ArmsUpStand {
        head: HeadMotion,
    },
    FallProtection {
        direction: FallingDirection,
    },
    Jump {
        direction: JumpDirection,
    },
    SitDown {
        head: HeadMotion,
    },
    Stand {
        head: HeadMotion,
    },
    Prepare,
    StandUp,
    KeeperMotion {
        direction: JumpDirection,
    },

    #[default]
    Unstiff,
    Animation {
        stiff: bool,
    },
    Walk {
        head: HeadMotion,
        path: Path,
        left_arm: ArmMotion,
        right_arm: ArmMotion,
        orientation_mode: OrientationMode,
        target_orientation: Orientation2<Ground>,
        distance_to_be_aligned: f32,
        speed: WalkSpeed,
    },
    InWalkKick {
        head: HeadMotion,
        left_arm: ArmMotion,
        right_arm: ArmMotion,
        kick: KickVariant,
        kicking_side: Side,
        strength: f32,
    },
    WalkWithVelocity {
        head: HeadMotion,
        velocity: Vector2<Ground>,
        angular_velocity: f32,
    },
    VisualKick {
        head: HeadMotion,
        ball_position: Point2<Ground>,
        kick_direction: Orientation2<Ground>,
        target_position: Point2<Ground>,
        robot_theta_to_field: Orientation2<Field>,
        kick_power: f64,
    },
}

impl MotionCommand {
    pub fn head_motion(&self) -> Option<HeadMotion> {
        match self {
            MotionCommand::ArmsUpStand { head }
            | MotionCommand::SitDown { head }
            | MotionCommand::Stand { head, .. }
            | MotionCommand::Walk { head, .. }
            | MotionCommand::InWalkKick { head, .. }
            | MotionCommand::WalkWithVelocity { head, .. }
            | MotionCommand::VisualKick { head, .. } => Some(*head),
            MotionCommand::Prepare => Some(HeadMotion::Center {
                image_region_target: ImageRegion::Top,
            }),
            MotionCommand::Unstiff => Some(HeadMotion::Unstiff),
            MotionCommand::Animation { stiff } => Some(HeadMotion::Animation { stiff: *stiff }),
            MotionCommand::ArmsUpSquat
            | MotionCommand::FallProtection { .. }
            | MotionCommand::Jump { .. }
            | MotionCommand::StandUp => None,
            MotionCommand::KeeperMotion { .. } => None,
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

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum HeadMotion {
    ZeroAngles,
    Center {
        image_region_target: ImageRegion,
    },
    LookAround,
    SearchForLostBall,
    LookAt {
        target: Point2<Ground>,
        image_region_target: ImageRegion,
    },
    LookAtReferee {
        image_region_target: ImageRegion,
    },
    LookLeftAndRightOf {
        target: Point2<Ground>,
    },
    Unstiff,
    Animation {
        stiff: bool,
    },
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum ImageRegion {
    Bottom,
    #[default]
    Center,
    Top,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum ArmMotion {
    Swing,
    PullTight,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum KickDirection {
    Back,
    Front,
    Left,
    Right,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum KickVariant {
    Forward,
    Turn,
    Side,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum JumpDirection {
    Left,
    Right,
    Center,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum GlanceDirection {
    #[default]
    LeftOfTarget,
    RightOfTarget,
}
