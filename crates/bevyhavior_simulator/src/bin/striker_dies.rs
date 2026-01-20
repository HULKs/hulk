use bevy::prelude::*;

use hsl_network_messages::{GameState, PlayerNumber};
use scenario::scenario;

use bevyhavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::roles::Role;

#[scenario]
fn striker_dies(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: MessageWriter<GameControllerCommand>,
) {
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
    game_controller_commands.write(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    mut commands: Commands,
    game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    mut exit: MessageWriter<AppExit>,
    robots: Query<(Entity, &Robot)>,
) {
    if time.ticks() == 5000 {
        robots
            .iter()
            .filter(|(_, robot)| robot.database.main_outputs.role == Role::Striker)
            .for_each(|(entity, _)| commands.entity(entity).despawn());
    }
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.write(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.write(AppExit::from_code(1));
    }
}
