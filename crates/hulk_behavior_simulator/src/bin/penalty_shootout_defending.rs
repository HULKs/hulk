use bevy::prelude::*;

use linear_algebra::{point, vector, Isometry2, Vector};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber, Team};
use types::{
    ball_position::SimulatorBallState,
    motion_command::{JumpDirection, MotionCommand},
};

use hulk_behavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn penalty_shootout_defending(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}
fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    mut ball: ResMut<BallResource>,
) {
    let mut robot = Robot::new(PlayerNumber::Two);
    *robot.ground_to_field_mut() = Isometry2::from_parts(vector![-4.5, 0.0], 0.0);
    commands.spawn(robot);
    ball.state = Some(SimulatorBallState {
        position: point!(-3.2, 0.0),
        velocity: Vector::zeros(),
    });
    game_controller_commands.send(GameControllerCommand::SetGamePhase(
        spl_network_messages::GamePhase::PenaltyShootout {
            kicking_team: Team::Opponent,
        },
    ));
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Set));
}

#[allow(clippy::too_many_arguments)]
fn update(
    game_controller: ResMut<GameController>,
    time: ResMut<Time<Ticks>>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
    mut robots: Query<&mut Robot>,
) {
    if time.ticks() == 2 {
        if let Some(ball) = ball.state.as_mut() {
            ball.velocity = vector![-2.5, -0.4];
        }
    }
    if time.ticks() > 2 {
        if let Some(ball) = ball.state.as_mut() {
            if ball.velocity.norm() < 0.01 {
                println!("Prevented opponent from scoring!");
                exit.send(AppExit::Success);
            }
            let robot = robots.single_mut();

            // basic collision physics
            let field_to_ground = robot.ground_to_field().inverse();
            let ball_in_ground = field_to_ground * ball.position;
            let velocity_in_ground = field_to_ground * ball.velocity;

            if let MotionCommand::Jump {
                direction: JumpDirection::Center,
            } = robot.database.main_outputs.motion_command
            {
                let x = ball_in_ground.x();
                let y = ball_in_ground.y();
                let ball_is_within_wide_stance = (x * x) / (0.04) + (y * y) / (0.16) < 1.0;
                if ball_is_within_wide_stance
                    && ball_in_ground
                        .coords()
                        .normalize()
                        .dot(velocity_in_ground.normalize())
                        < -0.3
                {
                    ball.velocity = vector!(0.2, 0.0);
                }
            };
        }
    }
    if game_controller.state.opponent_team.score > 0 {
        println!("Failed to prevent opponents from scoring");
        exit.send(AppExit::from_code(1));
    }
    if time.ticks() >= 10_000 {
        println!("Done");
        exit.send(AppExit::Success);
    }
}
