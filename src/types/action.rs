use approx::RelativeEq;
use macros::SerializeHierarchy;
use nalgebra::Translation2;
use serde::{Deserialize, Serialize};

use super::{
    motion_command::MotionCommand, ArmMotion, FallState, HeadMotion, InWalkKick, Motion,
    PrimaryState, WorldState,
};

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy)]
pub struct Action {
    #[leaf]
    pub action: Actions,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Actions {
    FallSafely,
    Penalize,
    Stand,
    StandUp,
    SitDown,
    Unstiff,
    WalkToPose,
}

impl Default for Actions {
    fn default() -> Self {
        Actions::WalkToPose
    }
}

impl Actions {
    pub fn is_available(&self, world_state: &WorldState) -> bool {
        match self {
            Self::FallSafely => matches!(world_state.robot.fall_state, FallState::Falling { .. }),
            Self::Unstiff => matches!(world_state.robot.primary_state, PrimaryState::Unstiff),
            Self::SitDown => matches!(world_state.robot.primary_state, PrimaryState::Finished),
            Self::Penalize => matches!(
                world_state.robot.primary_state,
                PrimaryState::Penalized | PrimaryState::Initial
            ),
            Self::Stand => {
                let relative_pose =
                    world_state.robot.pose.inverse() * world_state.robot.walk_target_pose;
                !world_state.robot.has_ground_contact
                    || relative_pose
                        .translation
                        .relative_eq(&Translation2::identity(), 0.05, 0.05)
                        && relative_pose.rotation.angle().relative_eq(&0.0, 0.3, 0.3)
            }
            Self::StandUp => matches!(world_state.robot.fall_state, FallState::Fallen { .. }),
            Self::WalkToPose => true,
        }
    }

    pub fn execute(&self, world_state: &WorldState) -> MotionCommand {
        let head = match world_state.ball.position {
            Some(target) => HeadMotion::LookAt { target },
            None => HeadMotion::LookAround,
        };
        match self {
            Self::FallSafely => match world_state.robot.fall_state {
                FallState::Falling { direction } => MotionCommand {
                    motion: Motion::FallProtection { direction },
                },
                _ => MotionCommand {
                    motion: Motion::Penalized, //throw hissy fit instead ?
                },
            },
            Self::Penalize => MotionCommand {
                motion: Motion::Penalized,
            },
            Self::Stand => MotionCommand {
                motion: Motion::Stand { head },
            },
            Self::StandUp => match world_state.robot.fall_state {
                FallState::Fallen { facing } => MotionCommand {
                    motion: Motion::StandUp { facing },
                },
                _ => MotionCommand {
                    motion: Motion::Penalized, //throw hissy fit instead ?
                },
            },
            Self::Unstiff => MotionCommand {
                motion: Motion::Unstiff,
            },
            Self::WalkToPose => MotionCommand {
                motion: Motion::Walk {
                    head,
                    in_walk_kick: InWalkKick::None,
                    left_arm: ArmMotion::Swing,
                    right_arm: ArmMotion::Swing,
                    target_pose: world_state.robot.pose.inverse()
                        * world_state.robot.walk_target_pose,
                },
            },
            Self::SitDown => MotionCommand {
                motion: Motion::SitDown {
                    head: HeadMotion::Unstiff,
                },
            },
        }
    }
}
