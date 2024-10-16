use std::time::Duration;

use bevy::prelude::*;
use spl_network_messages::{bindings::MAX_NUM_PLAYERS, Penalty, Team};

use crate::{game_controller::GameControllerCommand, robot::Robot};

pub fn hulks_aufstellung(
    active_field_players: Vec<usize>,
    picked_up_players: Vec<usize>,
    goal_keeper_jersey_number: usize,
    mut commands: Commands,
    game_controller_commands: &mut EventWriter<GameControllerCommand>,
) {
    let mut active_players_in_game_controller = active_field_players.clone();
    active_players_in_game_controller.append(&mut picked_up_players.clone());
    active_players_in_game_controller.sort();
    let mut index_array: Vec<usize> = Vec::with_capacity(active_field_players.len());

    for &jersey_number in &active_field_players {
        let mut index = active_players_in_game_controller
            .iter()
            .position(|&x| x == jersey_number)
            .unwrap();

        if jersey_number == goal_keeper_jersey_number {
            index_array.push(0);
        } else {
            if jersey_number < goal_keeper_jersey_number {
                index += 1;
            }
            index_array.push(index);
        }
    }
    for (jersey_number, walk_in_position_index) in
        active_field_players.iter().zip(index_array.iter())
    {
        commands.spawn(Robot::new(*jersey_number, *walk_in_position_index));
    }

    for jersey_number in picked_up_players {
        game_controller_commands.send(GameControllerCommand::Penalize(
            jersey_number,
            Penalty::RequestForPickup {
                remaining: Duration::MAX,
            },
        ));
    }
    for jersey_number in 1..=MAX_NUM_PLAYERS as usize {
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
