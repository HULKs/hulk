use linear_algebra::point;
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    field_dimensions::{FieldDimensions, Half},
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
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
            should_look_for_referee: false,
        }),
        PrimaryState::Set => {
            let ground_to_field = world_state.robot.ground_to_field?;
            let (fallback_target, is_opponent_penalty_kick) = match world_state
                .filtered_game_controller_state
            {
                Some(FilteredGameControllerState {
                    sub_state: Some(SubState::PenaltyKick),
                    kicking_team,
                    ..
                })
                | Some(FilteredGameControllerState {
                    game_phase: GamePhase::PenaltyShootout { .. },
                    kicking_team,
                    ..
                }) => match kicking_team {
                    Some(Team::Hulks) => (
                        world_state
                            .rule_ball
                            .map(|rule_ball| rule_ball.ball_in_ground)
                            .unwrap_or({
                                ground_to_field.inverse()
                                    * field_dimensions.penalty_spot(Half::Opponent)
                            }),
                        false,
                    ),
                    Some(Team::Opponent) => (
                        world_state
                            .rule_ball
                            .map(|rule_ball| rule_ball.ball_in_ground)
                            .unwrap_or({
                                ground_to_field.inverse() * field_dimensions.penalty_spot(Half::Own)
                            }),
                        true,
                    ),
                    _ => {
                        eprintln!("uncertain team during penalty kick or penalty shootout should not occur");
                        (point!(0.5, 0.0), true)
                    }
                },
                _ => (ground_to_field.inverse().as_pose().position(), false),
            };
            let target = world_state
                .ball
                .map(|state| state.ball_in_ground)
                .unwrap_or(fallback_target);
            match (role, is_opponent_penalty_kick) {
                (Role::Keeper, true) => Some(MotionCommand::ArmsUpStand {
                    head: HeadMotion::LookAt {
                        target,
                        image_region_target: Default::default(),
                        camera: None,
                    },
                }),
                _ => Some(MotionCommand::Stand {
                    head: HeadMotion::LookAt {
                        target,
                        image_region_target: Default::default(),
                        camera: None,
                    },
                    should_look_for_referee: false,
                }),
            }
        }
        PrimaryState::Playing => {
            match (
                &world_state.filtered_game_controller_state,
                world_state.robot.role,
                world_state.ball,
            ) {
                (
                    Some(
                        FilteredGameControllerState {
                            game_phase: GamePhase::PenaltyShootout { .. },
                            kicking_team,
                            ..
                        }
                        | FilteredGameControllerState {
                            game_phase: GamePhase::Normal,
                            kicking_team,
                            sub_state: Some(SubState::PenaltyKick),
                            ..
                        },
                    ),
                    Role::Striker,
                    None,
                ) => {
                    let ground_to_field = world_state.robot.ground_to_field?;
                    let target = match kicking_team {
                        Some(Team::Hulks) => world_state
                            .ball
                            .or(world_state.rule_ball)
                            .map(|ball| ball.ball_in_ground)
                            .unwrap_or({
                                ground_to_field.inverse()
                                    * field_dimensions.penalty_spot(Half::Opponent)
                            }),
                        Some(Team::Opponent) => world_state
                            .ball
                            .or(world_state.rule_ball)
                            .map(|ball| ball.ball_in_ground)
                            .unwrap_or({
                                ground_to_field.inverse() * field_dimensions.penalty_spot(Half::Own)
                            }),
                        _ => {
                            eprintln!("uncertain team during penalty kick or penalty shootout should not occur");
                            point!(0.5, 0.0)
                        }
                    };

                    Some(MotionCommand::Stand {
                        head: HeadMotion::LookAt {
                            target,
                            image_region_target: ImageRegion::Center,
                            camera: None,
                        },
                        should_look_for_referee: false,
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
}
