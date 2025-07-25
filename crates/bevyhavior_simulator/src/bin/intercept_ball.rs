use bevy::{ecs::system::SystemParam, prelude::*};

use linear_algebra::{point, vector, Isometry2, Point2, Vector};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};
use types::{ball_position::SimulatorBallState, step::Step};

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    soft_error::SoftErrorSender,
    time::{Ticks, TicksTime},
};

#[scenario]
fn intercept_ball(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

#[derive(SystemParam)]
struct State<'s> {
    count: Local<'s, usize>,
    previous_goal_count: Local<'s, u8>,
}

fn startup(
    mut commands: Commands,
    mut game_controller: ResMut<GameController>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    mut ball: ResMut<BallResource>,
) {
    let mut robot = Robot::new(PlayerNumber::One);
    *robot.ground_to_field_mut() = Isometry2::from_parts(vector![-2.0, 0.0], 0.0);
    robot.parameters.step_planner.max_step_size.forward = 1.0;
    robot.parameters.step_planner.max_step_size.left = 1.0;
    robot.parameters.step_planner.request_scale = Step {
        forward: 1.0,
        left: 1.0,
        turn: 1.0,
    };
    commands.spawn(robot);
    game_controller.state.game_state = GameState::Playing;
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Playing));
    ball.state = Some(SimulatorBallState {
        position: Point2::origin(),
        velocity: vector![2.0, 0.1],
    });
    ball.friction_coefficient = 0.999;
}

#[allow(clippy::too_many_arguments)]
fn update(
    game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
    mut robots: Query<&mut Robot>,
    mut state: State,
    mut soft_error: SoftErrorSender,
) {
    if let Some(ball) = ball.state.as_mut() {
        let mut robot = robots.single_mut();
        let field_dimensions = robot.parameters.field_dimensions;

        if ball.velocity.x() > 0.0 {
            robot.database.main_outputs.ground_to_field =
                Some(Isometry2::from_parts(vector![-4.0, 0.0], 0.0));
            ball.position = point![2.0, 0.0];
            let target = point![
                -field_dimensions.length / 2.0,
                field_dimensions.goal_inner_width * ((*state.count as f32 / 20.0) - 0.5) * 0.99
            ];
            ball.velocity = (target - ball.position).normalize() * 1.0;
            *state.count += 1;
        }

        // basic collision physics
        let field_to_ground = robot.ground_to_field().inverse();
        let ball_in_ground = field_to_ground * ball.position;
        let velocity_in_ground = field_to_ground * ball.velocity;

        if ball_in_ground.coords().norm() < 0.2
            && ball_in_ground
                .coords()
                .normalize()
                .dot(&velocity_in_ground.normalize())
                < -0.3
        {
            ball.velocity = Vector::zeros();
        }
    }

    if *state.previous_goal_count < game_controller.state.opponent_team.score {
        *state.previous_goal_count = game_controller.state.opponent_team.score;
        soft_error.send("Failed to prevent goals from being scored :(");
    }
    if *state.count > 20 {
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 20_000 {
        println!("Scenario timed out with insufficient balls held, please fix");
        exit.send(AppExit::from_code(2));
    }
}
