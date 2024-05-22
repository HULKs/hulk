use bevy::{
    app::AppExit,
    ecs::{
        event::EventWriter,
        system::{Commands, Query, ResMut},
    },
    time::Time,
};

use spl_network_messages::{GameState, PlayerNumber};
use types::ball_position::SimulatorBallState;

use hulk_behavior_simulator::{
    ball::BallResource,
    game_controller::GameController,
    robot::Robot,
    scenario,
    time::{Ticks, TicksTime},
};

scenario!(golden_goal);

fn golden_goal(
    mut robots: Query<&mut Robot>,
    mut commands: Commands,
    mut game_controller: ResMut<GameController>,
    mut ball: ResMut<BallResource>,
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
    if time.ticks() == 100 {
        for mut robot in &mut robots {
            robot.is_penalized = false;
            ball.state.as_ref().inspect(|ball| {
                println!("{:?}", ball.position);
            });
        }
        game_controller.state.game_state = GameState::Ready;
    }
    if time.ticks() == 1600 {
        game_controller.state.game_state = GameState::Set;
    }
    if time.ticks() == 1700 {
        ball.state = Some(SimulatorBallState::default());
    }
    if time.ticks() % 2000 == 0 {
        game_controller.state.game_state = GameState::Playing;
    }
    if ball
        .state
        .as_ref()
        .is_some_and(|ball| ball.position.x().abs() > 4.5 && ball.position.y() < 0.75)
        || time.ticks() >= 10_000
    {
        exit.send(AppExit);
        println!("Done");
    }
    if time.ticks() % 299 == 0 {
        println!("{}", time.ticks());
    }
}
