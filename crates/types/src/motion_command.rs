use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use linear_algebra::{Orientation2, Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::path::Path;

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
    #[default]
    Prepare,
    Stand {
        head: HeadMotion,
    },
    StandUp,
    VisualKick {
        head: HeadMotion,
        ball_position: Point2<Ground>,
        kick_direction: Orientation2<Ground>,
        target_position: Point2<Ground>,
        robot_theta_to_field: Orientation2<Field>,
        kick_power: KickPower,
    },
    Walk {
        head: HeadMotion,
        path: Path,
        orientation_mode: OrientationMode,
        target_orientation: Orientation2<Ground>,
        distance_to_be_aligned: f32,
        speed: f32,
    },
    WalkWithVelocity {
        head: HeadMotion,
        velocity: Vector2<Ground>,
        angular_velocity: f32,
    },
}

impl MotionCommand {
    pub fn head_motion(&self) -> Option<HeadMotion> {
        match self {
            MotionCommand::Stand { head, .. }
            | MotionCommand::Walk { head, .. }
            | MotionCommand::WalkWithVelocity { head, .. }
            | MotionCommand::VisualKick { head, .. } => Some(*head),
            MotionCommand::Prepare => Some(HeadMotion::Center {
                image_region_target: ImageRegion::Top,
            }),
            MotionCommand::StandUp => None,
        }
    }

    pub fn from_partial_motions(body: BodyMotion, head: HeadMotion) -> Self {
        match body {
            BodyMotion::Prepare => MotionCommand::Prepare,
            BodyMotion::Stand => MotionCommand::Stand { head },
            BodyMotion::StandUp => MotionCommand::StandUp,
            BodyMotion::VisualKick {
                ball_position,
                kick_direction,
                target_position,
                robot_theta_to_field,
                kick_power,
            } => MotionCommand::VisualKick {
                head,
                ball_position,
                kick_direction,
                target_position,
                robot_theta_to_field,
                kick_power,
            },
            BodyMotion::Walk {
                path,
                orientation_mode,
                target_orientation,
                distance_to_be_aligned,
                speed,
            } => MotionCommand::Walk {
                head,
                path,
                orientation_mode,
                target_orientation,
                distance_to_be_aligned,
                speed,
            },
            BodyMotion::WalkWithVelocity {
                velocity,
                angular_velocity,
            } => MotionCommand::WalkWithVelocity {
                head,
                velocity,
                angular_velocity,
            },
        }
    }
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
pub enum BodyMotion {
    #[default]
    Prepare,
    Stand,
    StandUp,
    VisualKick {
        ball_position: Point2<Ground>,
        kick_direction: Orientation2<Ground>,
        target_position: Point2<Ground>,
        robot_theta_to_field: Orientation2<Field>,
        kick_power: KickPower,
    },
    Walk {
        path: Path,
        orientation_mode: OrientationMode,
        target_orientation: Orientation2<Ground>,
        distance_to_be_aligned: f32,
        speed: f32,
    },
    WalkWithVelocity {
        velocity: Vector2<Ground>,
        angular_velocity: f32,
    },
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
pub enum KickPower {
    #[default]
    Rumpelstilzchen,
    Schlong,
}
