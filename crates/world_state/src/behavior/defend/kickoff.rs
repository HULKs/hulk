use coordinate_systems::Ground;
use framework::AdditionalOutput;
use linear_algebra::{distance, point, Point2, Pose2};
use types::{
    field_dimensions::FieldDimensions,
    motion_command::{MotionCommand, WalkSpeed},
    parameters::RolePositionsParameters,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::{core::Defend, left::block_on_circle};

impl<'cycle> Defend<'cycle> {
    pub fn kick_off(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        let pose =
            defend_kick_off_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
            self.walk_and_stand.parameters.defender_hysteresis,
        )
    }
}

fn defend_kick_off_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
) -> Option<Pose2<Ground>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let absolute_ball_position = match world_state.ball {
        Some(ball) => ball.ball_in_field,
        None => Point2::origin(),
    };
    let position_to_defend = point![-field_dimensions.length / 2.0, 0.0];
    let center_circle_radius = field_dimensions.center_circle_diameter / 2.0;
    let distance_to_target = distance(position_to_defend, absolute_ball_position)
        - center_circle_radius
        - role_positions.striker_distance_to_non_free_center_circle;
    let defend_pose = block_on_circle(
        absolute_ball_position,
        position_to_defend,
        distance_to_target,
    );
    Some(ground_to_field.inverse() * defend_pose)
}
