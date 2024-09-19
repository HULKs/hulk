use coordinate_systems::{Field, Ground};
use framework::AdditionalOutput;
use linear_algebra::{point, Isometry2, Orientation2, Point2, Pose2};
use types::{
    field_dimensions::FieldDimensions,
    motion_command::{HeadMotion, MotionCommand, OrientationMode, WalkSpeed},
    parameters::SearchParameters,
    path_obstacles::PathObstacle,
    roles::Role,
    support_foot::Side,
    world_state::WorldState,
};

use super::walk_to_pose::{WalkAndStand, WalkPathPlanner};

#[derive(Clone, Copy)]
enum SearchRole {
    Goal,
    Defend { side: Side },
    Center,
    Support { side: Side },
    Aggressive,
}

impl SearchRole {
    fn to_position(
        self,
        ground_to_field: Isometry2<Ground, Field>,
        field_dimensions: &FieldDimensions,
    ) -> Point2<Ground> {
        let goal = point![-field_dimensions.length / 2.0, 0.0];
        let defending_left = point![
            -field_dimensions.length / 2.0 + field_dimensions.goal_box_area_length + 0.2,
            field_dimensions.goal_inner_width / 4.0
        ];
        let defending_right = point![
            -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length + 0.2,
            -field_dimensions.goal_inner_width / 4.0
        ];
        let center = point![0.0, 0.0];
        let supporting_left = point![
            field_dimensions.goal_box_area_length + 0.2,
            field_dimensions.goal_inner_width / 4.0
        ];
        let supporting_right = point![
            field_dimensions.penalty_area_length + 0.2,
            -field_dimensions.goal_inner_width / 4.0
        ];
        let aggressive = point![
            field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
            0.0
        ];

        ground_to_field.inverse()
            * match self {
                SearchRole::Goal => goal,
                SearchRole::Defend { side: Side::Left } => defending_left,
                SearchRole::Defend { side: Side::Right } => defending_right,
                SearchRole::Center => center,
                SearchRole::Support { side: Side::Left } => supporting_left,
                SearchRole::Support { side: Side::Right } => supporting_right,
                SearchRole::Aggressive => aggressive,
            }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute(
    world_state: &WorldState,
    walk_path_planner: &WalkPathPlanner,
    walk_and_stand: &WalkAndStand,
    field_dimensions: &FieldDimensions,
    parameters: &SearchParameters,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    previous_role: Role,
    walk_speed: WalkSpeed,
    distance_to_be_aligned: f32,
) -> Option<MotionCommand> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let search_role = assign_search_role(world_state);
    let search_position = match (world_state.suggested_search_position, previous_role) {
        (Some(_), Role::Striker | Role::StrikerSupporter) => {
            ground_to_field.inverse() * world_state.suggested_search_position.unwrap()
        }
        _ => search_role
            .map(|role| role.to_position(ground_to_field, field_dimensions))
            .unwrap_or(point![0.0, 0.0]),
    };

    let best_hypothetical_ball_position = world_state
        .hypothetical_ball_positions
        .iter()
        .max_by(|a, b| a.validity.total_cmp(&b.validity));

    let head = match best_hypothetical_ball_position {
        Some(hypothesis) => HeadMotion::LookAt {
            target: hypothesis.position,
            image_region_target: Default::default(),
            camera: None,
        },
        None => HeadMotion::SearchForLostBall,
    };
    if let Some(SearchRole::Goal) = search_role {
        let goal_pose = Pose2::from(search_position);
        walk_and_stand.execute(
            goal_pose,
            head,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
        )
    } else {
        let path = walk_path_planner.plan(
            search_position,
            ground_to_field,
            None,
            1.0,
            &world_state.obstacles,
            &world_state.rule_obstacles,
            path_obstacles_output,
        );
        let path_length: f32 = path.iter().map(|segment| segment.length()).sum();
        let is_reached = path_length < parameters.position_reached_distance;
        let orientation_mode = if is_reached {
            OrientationMode::Override(Orientation2::new(parameters.rotation_per_step))
        } else {
            OrientationMode::AlignWithPath
        };
        Some(walk_path_planner.walk_with_obstacle_avoiding_arms(
            head,
            orientation_mode,
            path,
            walk_speed,
        ))
    }
}

fn assign_search_role(world_state: &WorldState) -> Option<SearchRole> {
    let search_roles = [
        SearchRole::Goal,
        SearchRole::Defend { side: Side::Left },
        SearchRole::Defend { side: Side::Right },
        SearchRole::Center,
        SearchRole::Support { side: Side::Left },
        SearchRole::Support { side: Side::Right },
        SearchRole::Aggressive,
    ]
    .into_iter();
    let penalties = world_state
        .filtered_game_controller_state
        .as_ref()
        .map(|state| state.penalties.clone())?;
    let available_players = penalties
        .iter()
        .filter_map(|(number, penalty)| match penalty {
            Some(_) => None,
            None => Some(number),
        });

    available_players
        .zip(search_roles)
        .find_map(|(number, position)| {
            let is_my_player_number = *number == world_state.robot.jersey_number;
            is_my_player_number.then_some(position)
        })
}
