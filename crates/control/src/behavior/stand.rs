use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    field_dimensions::{FieldDimensions, Half},
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{HeadMotion, MotionCommand},
    primary_state::PrimaryState,
    roles::Role,
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role: &Role,
) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Initial => Some(MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
        }),
        PrimaryState::Set => {
            let ground_to_field = world_state.robot.ground_to_field?;
            let mut kicking_team_in_set = Team::Uncertain;
            let mut is_penalty_kick = false;
            let fallback_target = match world_state.filtered_game_controller_state {
                Some(FilteredGameControllerState {
                    sub_state: Some(SubState::PenaltyKick),
                    kicking_team,
                    ..
                })
                | Some(FilteredGameControllerState {
                    game_phase: GamePhase::PenaltyShootout { .. },
                    kicking_team,
                    ..
                }) => {
                    kicking_team_in_set = kicking_team;
                    is_penalty_kick = true;
                    let half = match kicking_team {
                        Team::Hulks => Half::Opponent,
                        Team::Opponent => Half::Own,
                        Team::Uncertain => {
                            eprintln!("uncertain team during penalty kick or penalty shootout should not occur");
                            Half::Opponent
                        }
                    };
                    world_state
                        .rule_ball
                        .map(|rule_ball| rule_ball.ball_in_ground)
                        .unwrap_or({
                            ground_to_field.inverse() * field_dimensions.penalty_spot(half)
                        })
                }
                _ => ground_to_field.inverse().as_pose().position(),
            };
            let target = world_state
                .ball
                .map(|state| state.ball_in_ground)
                .unwrap_or(fallback_target);
            match (role, kicking_team_in_set, is_penalty_kick) {
                (Role::Keeper, Team::Opponent | Team::Uncertain, true) => {
                    Some(MotionCommand::ArmsUpStand {
                        head: HeadMotion::LookAt {
                            target,
                            image_region_target: Default::default(),
                            camera: None,
                        },
                    })
                }
                _ => Some(MotionCommand::Stand {
                    head: HeadMotion::LookAt {
                        target,
                        image_region_target: Default::default(),
                        camera: None,
                    },
                }),
            }
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
