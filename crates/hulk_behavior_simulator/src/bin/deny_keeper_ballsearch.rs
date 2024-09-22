use std::{f32::consts::FRAC_PI_2, time::Duration};

use bevy::prelude::*;

use linear_algebra::{vector, Isometry2};
use scenario::scenario;
use spl_network_messages::{GameState, Penalty};
use types::action::Action;

use hulk_behavior_simulator::{
    ball::BallResource,
    game_controller::GameControllerCommand,
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn deny_keeper_ballsearch(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    for number in 8..=20 {
        game_controller_commands.send(GameControllerCommand::Penalize(
            number,
            Penalty::Substitute {
                remaining: Duration::MAX,
            },
        ));
    }
    for number in 1..=7 {
        commands.spawn(Robot::new(number));
    }
    for number in 2..=6 {
        game_controller_commands.send(GameControllerCommand::Penalize(
            number,
            Penalty::RequestForPickup {
                remaining: Duration::MAX,
            },
        ));
    }
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
    mut robots: Query<&mut Robot>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
) {
    if time.ticks() == 3000 {
        ball.state = None;
    }

    // Penalize all except golkipör
    if time.ticks() == 4000 {
        let penalty = spl_network_messages::Penalty::Manual {
            remaining: Duration::from_secs(5),
        };
        for jersey_number in [2, 3, 4, 5, 6, 7] {
            game_controller_commands.send(GameControllerCommand::Penalize(jersey_number, penalty));
        }
        robots
            .iter_mut()
            .find(|robot| robot.parameters.jersey_number == 7)
            .unwrap()
            .database
            .main_outputs
            .ground_to_field = Some(Isometry2::from_parts(vector![-3.2, -3.3], FRAC_PI_2));
    }

    if robots
        .iter_mut()
        .find(|robot| robot.parameters.jersey_number == 1)
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
