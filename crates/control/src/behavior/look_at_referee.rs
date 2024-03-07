use nalgebra::Point2;
use spl_network_messages::PlayerNumber;
use types::{
    camera_position::CameraPosition,
    motion_command::{HeadMotion, MotionCommand},
    world_state::WorldState,
};

pub fn execute(
    world_state: &WorldState,
    expected_referee_position: Point2<f32>,
) -> Option<MotionCommand> {
    if let Some(game_controller_state) = world_state.filtered_game_controller_state {
        match game_controller_state.own_team_is_home_after_coin_toss {
            true => match world_state.robot.player_number {
                PlayerNumber::Seven | PlayerNumber::Four | PlayerNumber::Five => {
                    Some(MotionCommand::Stand {
                        head: HeadMotion::LookAt {
                            target: expected_referee_position,
                            camera: Some(CameraPosition::Top),
                        },
                    })
                }
                _ => None,
            },
            false => match world_state.robot.player_number {
                PlayerNumber::One | PlayerNumber::Two | PlayerNumber::Six => {
                    Some(MotionCommand::Stand {
                        head: HeadMotion::LookAt {
                            target: expected_referee_position,
                            camera: Some(CameraPosition::Top),
                        },
                    })
                }
                _ => None,
            },
        }
    } else {
        None
    }
}
