use linear_algebra::point;
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{HeadMotion, ImageRegionTarget, MotionCommand},
    primary_state::PrimaryState,
    roles::Role,
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Initial => Some(MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
        }),
        PrimaryState::Set => {
            let ground_to_field = world_state.robot.ground_to_field?;
            let fallback_target = match world_state.filtered_game_controller_state {
                Some(FilteredGameControllerState {
                    sub_state: Some(SubState::PenaltyKick),
                    kicking_team,
                    ..
                }) => {
                    let side_factor = match kicking_team {
                        Team::Opponent => -1.0,
                        _ => 1.0,
                    };
                    let penalty_spot_x =
                        field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance;
                    let penalty_spot_location = point![side_factor * penalty_spot_x, 0.0];
                    ground_to_field.inverse() * penalty_spot_location
                }
                _ => ground_to_field.inverse().as_pose().position(),
            };
            let target = world_state
                .ball
                .map(|state| state.ball_in_ground)
                .unwrap_or(fallback_target);
            Some(MotionCommand::Stand {
                head: HeadMotion::LookAt {
                    target,
                    pixel_target: ImageRegionTarget::Center,
                    camera: None,
                },
            })
        }
        PrimaryState::Playing => {
            match (
                world_state.filtered_game_controller_state,
                world_state.robot.role,
                world_state.ball,
            ) {
                (
                    Some(FilteredGameControllerState {
                        game_phase: GamePhase::PenaltyShootout { .. },
                        ..
                    }),
                    Role::Striker,
                    None,
                ) => Some(MotionCommand::Stand {
                    head: HeadMotion::Center,
                }),
                _ => None,
            }
        }
        _ => None,
    }
}
