use ros_z::Message;
use serde::{Deserialize, Serialize};

use coordinate_systems::Ground;
use linear_algebra::{Orientation2, Point2, Vector2};

use crate::path::Path;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Message)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Message)]
pub enum MotionCommand {
    #[default]
    Damping,
    Prepare,
    Stand {
        head: HeadMotion,
    },
    StandUp {
        fast: bool,
    },
    Kick {
        head: HeadMotion,
        /// Desired outgoing ball speed in m/s; inference applies policy limits.
        target_speed: f32,
        /// Select the soft-kick policy, which ignores strong and quick flags.
        soft: bool,
        quick: bool,
        strong: bool,
        ball_position: Point2<Ground>,
        /// Current ball velocity in Ground coordinates, in m/s.
        ball_velocity: Vector2<Ground>,
        kick_direction: Orientation2<Ground>,
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
            | MotionCommand::Kick { head, .. } => Some(*head),
            MotionCommand::Prepare => Some(HeadMotion::Center {
                image_region_target: ImageRegion::Top,
            }),
            MotionCommand::Damping | MotionCommand::StandUp { .. } => None,
        }
    }

    pub fn from_partial_motions(body: BodyMotion, head: HeadMotion) -> Self {
        match body {
            BodyMotion::Damping => MotionCommand::Damping,
            BodyMotion::Prepare => MotionCommand::Prepare,
            BodyMotion::Stand => MotionCommand::Stand { head },
            BodyMotion::StandUp { fast } => MotionCommand::StandUp { fast },
            BodyMotion::Kick {
                ball_position,
                ball_velocity,
                target_speed,
                soft,
                quick,
                kick_direction,
                strong,
            } => MotionCommand::Kick {
                head,
                ball_position,
                ball_velocity,
                target_speed,
                soft,
                quick,
                kick_direction,
                strong,
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Message)]
pub enum BodyMotion {
    #[default]
    Damping,
    Prepare,
    Stand,
    StandUp {
        fast: bool,
    },
    Kick {
        ball_position: Point2<Ground>,
        /// Current ball velocity in Ground coordinates, in m/s.
        ball_velocity: Vector2<Ground>,
        /// Desired outgoing ball speed in m/s; inference applies policy limits.
        target_speed: f32,
        /// Select the soft-kick policy, which ignores strong and quick flags.
        soft: bool,
        quick: bool,
        strong: bool,
        kick_direction: Orientation2<Ground>,
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, Message)]
pub enum HeadMotion {
    ZeroAngles,
    /// Frame the ground point half a field width straight ahead in Ground coordinates.
    Center {
        image_region_target: ImageRegion,
    },
    LookAround,
    SearchForLostBall,
    LookAt {
        target: Point2<Ground>,
        /// Height of the point of interest along Ground's +Z axis, in meters.
        height_above_ground: f32,
        image_region_target: ImageRegion,
    },
    LookLeftAndRightOf {
        target: Point2<Ground>,
        /// Height in meters along Ground's +Z, retained while offsetting either side.
        height_above_ground: f32,
    },
    Damping,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, Message)]
pub enum ImageRegion {
    Bottom,
    #[default]
    Center,
    Top,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damping_has_no_head_motion() {
        assert_eq!(MotionCommand::Damping.head_motion(), None);
    }

    #[test]
    fn body_damping_assembles_to_motion_damping() {
        let command = MotionCommand::from_partial_motions(
            BodyMotion::Damping,
            HeadMotion::Center {
                image_region_target: ImageRegion::Center,
            },
        );

        assert_eq!(command, MotionCommand::Damping);
    }
}
