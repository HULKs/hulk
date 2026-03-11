use coordinate_systems::Field;
use framework::AdditionalOutput;
use hsl_network_messages::GamePhase;
use linear_algebra::{Orientation2, Vector2, vector};
use types::{
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{HeadMotion, ImageRegion, MotionCommand, OrientationMode},
    parameters::KickingParameters,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use super::walk_to_pose::WalkPathPlanner;

pub fn execute(
    world_state: &WorldState,
    walk_path_planner: &WalkPathPlanner,
    parameters: &KickingParameters,
    walk_speed: f32,
    distance_to_be_aligned: f32,
    field_dimensions: FieldDimensions,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
) -> Option<MotionCommand> {
    let ball_position = world_state.ball?.ball_in_ground;
    let ground_to_field = world_state.robot.ground_to_field?;
    let distance_to_ball = ball_position.coords().norm();
    let head = if distance_to_ball < parameters.distance_to_look_directly_at_the_ball {
        HeadMotion::LookAt {
            target: ball_position,
            image_region_target: ImageRegion::Center,
        }
    } else {
        HeadMotion::LookLeftAndRightOf {
            target: ball_position,
        }
    };

    let goal_position: Vector2<Field> = vector!(field_dimensions.length / 2.0, 0.0);
    let field_to_ground = ground_to_field.inverse();
    let kick_direction =
        Orientation2::from_vector(field_to_ground * goal_position - ball_position.coords());

    let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();
    let target_position = (field_to_ground * goal_position).as_point();

    if false {
        Some(MotionCommand::VisualKick {
            head,
            ball_position,
            kick_direction,
            target_position,
            robot_theta_to_field,
            kick_power: parameters.kick_power,
        })
    } else {    
        let mut speed = walk_speed;
        if let Some(FilteredGameControllerState {
            game_phase: GamePhase::PenaltyShootout { .. },
            ..
        }) = world_state.filtered_game_controller_state
        {
            speed = 0.5;
        }

        let path = walk_path_planner.plan(
            ball_position,
            ground_to_field,
            None,
            1.0,
            &world_state.obstacles,
            &world_state.rule_obstacles,
            path_obstacles_output,
        );
        Some(walk_path_planner.walk_with_obstacle_avoiding_arms(
            head,
            OrientationMode::AlignWithPath,
            Orientation2::identity(),   
            distance_to_be_aligned,
            path,
            speed,
        ))
    }
}
