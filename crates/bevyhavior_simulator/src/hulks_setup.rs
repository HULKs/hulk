use std::time::Duration;

use bevy::prelude::*;
use spl_network_messages::{bindings::MAX_NUM_PLAYERS, Penalty, Team};

use crate::{game_controller::GameControllerCommand, robot::Robot};

pub fn hulks_setup(
    active_field_players: Vec<u8>,
    picked_up_players: Vec<u8>,
    goal_keeper_jersey_number: u8,
    mut commands: Commands,
    game_controller_commands: &mut EventWriter<GameControllerCommand>,
) {
    let mut active_players_in_game_controller = active_field_players.clone();
    active_players_in_game_controller.append(&mut picked_up_players.clone());

    for jersey_number in active_field_players.iter() {
        commands.spawn(Robot::new(*jersey_number, 0));
    }

    for jersey_number in picked_up_players {
        game_controller_commands.send(GameControllerCommand::Penalize(
            jersey_number,
            Penalty::RequestForPickup {
                remaining: Duration::MAX,
            },
        ));
    }
    for jersey_number in 1..=MAX_NUM_PLAYERS {
        if !active_players_in_game_controller.contains(&jersey_number) {
            game_controller_commands.send(GameControllerCommand::Penalize(
                jersey_number,
                Penalty::Substitute {
                    remaining: Duration::MAX,
                },
            ));
        }
    }
    game_controller_commands.send(GameControllerCommand::SetKeeperNumber(
        goal_keeper_jersey_number,
        Team::Hulks,
    ));
}
