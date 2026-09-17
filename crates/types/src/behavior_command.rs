use ros_z::Message;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
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
pub enum BehaviorCommand {
    #[default]
    Damping,
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

impl BehaviorCommand {
    pub fn head_motion(&self) -> Option<HeadMotion> {
        match self {
            BehaviorCommand::Stand { head, .. }
            | BehaviorCommand::Walk { head, .. }
            | BehaviorCommand::WalkWithVelocity { head, .. }
            | BehaviorCommand::VisualKick { head, .. } => Some(*head),
            BehaviorCommand::Prepare => Some(HeadMotion::Center {
                image_region_target: ImageRegion::Top,
            }),
            BehaviorCommand::Damping | BehaviorCommand::StandUp => None,
        }
    }

    pub fn from_partial_motions(body: BodyMotion, head: HeadMotion) -> Self {
        match body {
            BodyMotion::Damping => BehaviorCommand::Damping,
            BodyMotion::Prepare => BehaviorCommand::Prepare,
            BodyMotion::Stand => BehaviorCommand::Stand { head },
            BodyMotion::StandUp => BehaviorCommand::StandUp,
            BodyMotion::VisualKick {
                ball_position,
                kick_direction,
                target_position,
                robot_theta_to_field,
                kick_power,
            } => BehaviorCommand::VisualKick {
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
            } => BehaviorCommand::Walk {
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
            } => BehaviorCommand::WalkWithVelocity {
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, Message)]
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
    LookLeftAndRightOf {
        target: Point2<Ground>,
    },
    Unstiff,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, Message)]
pub enum ImageRegion {
    Bottom,
    #[default]
    Center,
    Top,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum GlanceDirection {
    #[default]
    LeftOfTarget,
    RightOfTarget,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Message)]
pub enum KickPower {
    #[default]
    Rumpelstilzchen,
    Schlong,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damping_has_no_head_motion() {
        assert_eq!(BehaviorCommand::Damping.head_motion(), None);
    }

    #[test]
    fn body_damping_assembles_to_behavior_damping() {
        let command = BehaviorCommand::from_partial_motions(
            BodyMotion::Damping,
            HeadMotion::Center {
                image_region_target: ImageRegion::Center,
            },
        );

        assert_eq!(command, BehaviorCommand::Damping);
    }
}
