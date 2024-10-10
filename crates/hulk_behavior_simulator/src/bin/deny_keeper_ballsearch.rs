use std::time::Duration;

use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::GameState;
use types::action::Action;

use hulk_behavior_simulator::{
    aufstellung::hulks_aufstellung,
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

fn startup(commands: Commands, mut game_controller_commands: EventWriter<GameControllerCommand>) {
    let active_field_players = vec![1, 2, 3, 4, 5, 6, 7];
    let picked_up_players = vec![];
    let goal_keeper_jersey_number = 1;
    hulks_aufstellung(
        active_field_players,
        picked_up_players,
        goal_keeper_jersey_number,
        commands,
        &mut game_controller_commands,
    );
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
        let penalty = spl_network_messages::Penalty::RequestForPickup {
            remaining: Duration::from_secs(5),
        };
        for jersey_number in [2, 3, 4, 5, 6, 7] {
            game_controller_commands.send(GameControllerCommand::Penalize(jersey_number, penalty));
        }
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
