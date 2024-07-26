use bevy::{
    app::AppExit,
    ecs::{
        event::EventWriter,
        system::{Commands, Res, ResMut},
    },
    time::Time,
};

use spl_network_messages::{GameState, PlayerNumber};

use hulk_behavior_simulator::{
    game_controller::GameController,
    robot::Robot,
    scenario,
    time::{Ticks, TicksTime},
};

scenario!(golden_goal);

fn golden_goal(
    mut commands: Commands,
    mut game_controller: ResMut<GameController>,
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
            let robot = Robot::try_new(number).unwrap();
            commands.spawn(robot);
        }
        game_controller.state.game_state = GameState::Ready;
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
