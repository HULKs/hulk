use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use bevyhavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use geometry::{arc::Arc, circle::Circle, direction::Direction, line_segment::LineSegment};
use linear_algebra::{point, vector, Isometry2, Orientation2, Point2};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};
use types::{
    motion_command::{ArmMotion, HeadMotion, MotionCommand, OrientationMode, WalkSpeed},
    planned_path::{Path, PathSegment},
};

#[scenario]
fn mpc_step_planner_optimizer(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    commands.spawn(Robot::new(PlayerNumber::Seven));

    game_controller_commands.write(GameControllerCommand::SetGameState(GameState::Playing));
}

fn update(
    _game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
    mut robots: Query<&mut Robot>,
) {
    let mut robot = robots.single_mut().expect("no robot found");

    robot.database.main_outputs.ground_to_field =
        Some(Isometry2::from_parts(vector![-1.0, -1.0], FRAC_PI_2));
    robot.parameters.behavior.injected_motion_command = Some(MotionCommand::Walk {
        head: HeadMotion::ZeroAngles,
        left_arm: ArmMotion::Swing,
        right_arm: ArmMotion::Swing,
        speed: WalkSpeed::Normal,
        path: Path {
            segments: vec![
                PathSegment::LineSegment(LineSegment(Point2::origin(), point![0.3, 0.0])),
                PathSegment::Arc(Arc {
                    circle: Circle {
                        center: point![0.3, 0.3],
                        radius: 0.3,
                    },
                    start: Orientation2::new(3.0 * FRAC_PI_2),
                    end: Orientation2::new(0.0),
                    direction: Direction::Counterclockwise,
                }),
                PathSegment::LineSegment(LineSegment(point![0.6, 0.3], point![0.6, 0.8])),
            ],
        },
        orientation_mode: OrientationMode::Unspecified,
        target_orientation: Orientation2::identity(),
        distance_to_be_aligned: 0.1,
    });

    let optimizer_steps = time.ticks() as usize;

    println!("tick {}: {optimizer_steps} steps", time.ticks());

    if time.ticks() >= 500 {
        exit.write(AppExit::Success);
    }
}
