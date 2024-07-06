use bevy::{
    app::AppExit,
    ecs::{
        event::EventWriter,
        system::{Commands, Res, ResMut},
    },
    time::Time,
};

use spl_network_messages::PlayerNumber;

use hulk_behavior_simulator::{
    game_controller::GameController,
    robot::Robot,
    scenario,
    time::{Ticks, TicksTime},
};

scenario!(golden_goal);

fn golden_goal(
    mut commands: Commands,
    game_controller: Res<GameController>,
    time: ResMut<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
) {
    if time.ticks() == 1 {
        commands.spawn(Robot::try_new(PlayerNumber::One).unwrap());
        commands.spawn(Robot::try_new(PlayerNumber::Two).unwrap());
        commands.spawn(Robot::try_new(PlayerNumber::Three).unwrap());
        commands.spawn(Robot::try_new(PlayerNumber::Four).unwrap());
        commands.spawn(Robot::try_new(PlayerNumber::Five).unwrap());
        commands.spawn(Robot::try_new(PlayerNumber::Six).unwrap());
        commands.spawn(Robot::try_new(PlayerNumber::Seven).unwrap());
    }
    if game_controller.state.hulks_team.score > 4 || time.ticks() >= 100_000 {
        exit.send(AppExit);
        println!("Done");
    }
    if time.ticks() % 299 == 0 {
        println!("{}", time.ticks());
    }
}
