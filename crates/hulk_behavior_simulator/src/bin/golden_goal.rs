use bevy::prelude::*;

use spl_network_messages::{GameState, PlayerNumber};

use hulk_behavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    scenario,
    time::{Ticks, TicksTime},
};

scenario!(golden_goal, |app: &mut App| {
    app.add_systems(Update, update);
});

fn update(
    mut commands: Commands,
    game_controller: ResMut<GameController>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
) {
    if time.ticks() == 1 {
        for number in [
            PlayerNumber::One,
            PlayerNumber::Two,
            PlayerNumber::Three,
            PlayerNumber::Four,
            PlayerNumber::Five,
            PlayerNumber::Six,
            PlayerNumber::Seven,
        ] {
            commands.spawn(Robot::new(number));
        }
        game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    }

    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
