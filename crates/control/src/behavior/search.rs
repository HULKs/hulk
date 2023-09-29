use framework::AdditionalOutput;
use nalgebra::{point, Isometry2, Point2, UnitComplex};
use types::{
    field_dimensions::FieldDimensions,
    motion_command::{HeadMotion, MotionCommand, OrientationMode},
    parameters::SearchParameters,
    path_obstacles::PathObstacle,
    support_foot::Side,
    world_state::WorldState,
};

use super::walk_to_pose::{WalkAndStand, WalkPathPlanner};

#[derive(Clone, Copy)]
enum SearchRole {
    Goal,
    Defend { side: Side },
    Center,
    Aggressive,
}

impl SearchRole {
    fn to_position(
        self,
        robot_to_field: Isometry2<f32>,
        field_dimensions: &FieldDimensions,
    ) -> Point2<f32> {
        let goal = point![-field_dimensions.length / 2.0, 0.0];
        let defending_left = point![
            -field_dimensions.length / 2.0 + field_dimensions.goal_box_area_length + 0.2,
            field_dimensions.goal_box_area_width / 2.0
        ];
        let defending_right = point![
            -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length + 0.2,
            -field_dimensions.penalty_area_width / 2.0
        ];
        let center = point![0.0, 0.0];
        let aggressive = point![
            field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
            0.0
        ];

        robot_to_field.inverse()
            * match self {
                SearchRole::Goal => goal,
                SearchRole::Defend { side: Side::Left } => defending_left,
                SearchRole::Defend { side: Side::Right } => defending_right,
                SearchRole::Center => center,
                SearchRole::Aggressive => aggressive,
            }
    }
}

pub fn execute(
    world_state: &WorldState,
    walk_path_planner: &WalkPathPlanner,
    walk_and_stand: &WalkAndStand,
    field_dimensions: &FieldDimensions,
    parameters: &SearchParameters,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<MotionCommand> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let search_role = assign_search_role(world_state);
    let search_position = search_role
        .map(|role| role.to_position(robot_to_field, field_dimensions))
        .unwrap_or(point![0.0, 0.0]);
    let head = HeadMotion::SearchForLostBall;
    if let Some(SearchRole::Goal) = search_role {
        let goal_pose = robot_to_field.inverse() * Isometry2::from(search_position.coords);
        walk_and_stand.execute(goal_pose, head, path_obstacles_output)
    } else {
        let path = walk_path_planner.plan(
            search_position,
            robot_to_field,
            None,
            1.0,
            &world_state.obstacles,
            &world_state.rule_obstacles,
            path_obstacles_output,
        );
        let path_length: f32 = path.iter().map(|segment| segment.length()).sum();
        let is_reached = path_length < parameters.position_reached_distance;
        let orientation_mode = if is_reached {
            OrientationMode::Override(UnitComplex::new(parameters.rotation_per_step))
        } else {
            OrientationMode::AlignWithPath
        };
        Some(walk_path_planner.walk_with_obstacle_avoiding_arms(head, orientation_mode, path))
    }
}

fn assign_search_role(world_state: &WorldState) -> Option<SearchRole> {
    let search_roles = [
        SearchRole::Goal,
        SearchRole::Defend { side: Side::Left },
        SearchRole::Defend { side: Side::Right },
        SearchRole::Center,
        SearchRole::Aggressive,
    ]
    .into_iter();
    let penalties = world_state
        .game_controller_state
        .map(|state| state.penalties)?;
    let available_players = penalties
        .iter()
        .filter_map(|(number, penalty)| match penalty {
            Some(_) => None,
            None => Some(number),
        });

    available_players
        .zip(search_roles)
        .find_map(|(number, position)| {
            let is_my_player_number = number == world_state.robot.player_number;
            is_my_player_number.then_some(position)
        })
}
