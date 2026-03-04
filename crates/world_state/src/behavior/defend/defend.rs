
use coordinate_systems::Ground;
use framework::AdditionalOutput;
use linear_algebra::Pose2;
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions,
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    parameters::RolePositionsParameters,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::super::{walk_to_pose::WalkAndStand, head::LookAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefendMode {
    Aggressive,
    Passive,
}

pub struct Defend<'cycle> {
    pub world_state: &'cycle WorldState,
    pub field_dimensions: &'cycle FieldDimensions,
    pub role_positions: &'cycle RolePositionsParameters,
    pub walk_and_stand: &'cycle WalkAndStand<'cycle>,
    pub look_action: &'cycle LookAction<'cycle>,
    pub last_defender_mode: &'cycle mut DefendMode,
}

impl<'cycle> Defend<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        field_dimensions: &'cycle FieldDimensions,
        role_positions: &'cycle RolePositionsParameters,
        walk_and_stand: &'cycle WalkAndStand,
        look_action: &'cycle LookAction,
        last_defender_mode: &'cycle mut DefendMode,
    ) -> Self {
        Self {
            world_state,
            field_dimensions,
            role_positions,
            walk_and_stand,
            look_action,
            last_defender_mode,
        }
    }

    pub fn with_pose(
        &self,
        pose: Pose2<Ground>,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        distance_to_be_aligned: f32,
        hysteresis: nalgebra::Vector2<f32>,
    ) -> Option<MotionCommand> {
        self.walk_and_stand.execute(
            pose,
            self.look_action.execute(),
            path_obstacles_output,
            walk_speed,
            // TODO(rmburg): maybe change this instead of having a large distance_to_be_aligned?
            OrientationMode::AlignWithPath,
            distance_to_be_aligned,
            hysteresis,
        )
    }

}
