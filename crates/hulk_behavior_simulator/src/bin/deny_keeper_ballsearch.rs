use std::{f32::consts::FRAC_PI_2, time::Duration};

use bevy::prelude::*;

use linear_algebra::{vector, Isometry2};
use spl_network_messages::{GameState, PlayerNumber};
use types::action::Action;

use hulk_behavior_simulator::{
    ball::BallResource,
    game_controller::GameControllerCommand,
    robot::Robot,
    scenario,
    time::{Ticks, TicksTime},
};

scenario!(deny_keeper_ballsearch, |app: &mut App| {
    app.add_systems(Update, update);
});

fn update(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
    mut robots: Query<&mut Robot>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
) {
    if time.ticks() == 1 {
        for number in [PlayerNumber::One, PlayerNumber::Seven] {
            commands.spawn(Robot::new(number));
        }
        game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    }

    if time.ticks() == 3000 {
        ball.state = None;
    }

    // Penalize all except golkipör
    if time.ticks() == 4000 {
        let penalty = spl_network_messages::Penalty::Manual {
            remaining: Duration::from_secs(5),
        };
        for player_numer in [
            PlayerNumber::Two,
            PlayerNumber::Three,
            PlayerNumber::Four,
            PlayerNumber::Five,
            PlayerNumber::Six,
            PlayerNumber::Seven,
        ] {
            game_controller_commands.send(GameControllerCommand::Penalize(player_numer, penalty));
        }
        robots
            .iter_mut()
            .find(|robot| robot.parameters.player_number == PlayerNumber::Seven)
            .unwrap()
            .database
            .main_outputs
            .ground_to_field = Some(Isometry2::from_parts(vector![-3.2, -3.3], FRAC_PI_2));
    }

    if robots
        .iter_mut()
        .find(|robot| robot.parameters.player_number == PlayerNumber::One)
        .and_then(|robot| robot.database.additional_outputs.active_action)
        == Some(Action::Search)
    {
        println!("Keeper tried to enter ball search");
        exit.send(AppExit::from_code(1));
    }
    if time.ticks() >= 10_000 {
        println!("Done");
        exit.send(AppExit::Success);
    }
}
