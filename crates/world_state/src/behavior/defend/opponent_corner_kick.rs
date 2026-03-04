use framework::AdditionalOutput;
use types::{field_dimensions::Side, motion_command::{MotionCommand, WalkSpeed}, path_obstacles::PathObstacle};

use super::{defend::Defend, left::defend_pose};

impl<'cycle> Defend<'cycle> {
    pub fn opponent_corner_kick(
        &mut self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        field_side: Side,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        let pose = defend_pose(
            self.world_state,
            self.field_dimensions,
            self.role_positions,
            -self.field_dimensions.length / 2.0 + self.field_dimensions.goal_box_area_length * 2.0,
            field_side,
            self.last_defender_mode,
        )?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
            self.walk_and_stand.parameters.defender_hysteresis,
        )
    }
}