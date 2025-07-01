use std::f32::consts::TAU;

use bevy::prelude::*;

use bevyhavior_simulator::{
    game_controller::GameControllerCommand,
    robot::Robot,
    time::{Ticks, TicksTime},
};
use linear_algebra::{point, Isometry2, Orientation2, Point2};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};
use types::{
    motion_command::{ArmMotion, HeadMotion, MotionCommand, OrientationMode, WalkSpeed},
    planned_path::{Path, PathSegment},
};

#[scenario]
fn demonstration(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    commands.spawn(Robot::new(PlayerNumber::Seven));
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Playing));
}

fn update(time: Res<Time<Ticks>>, mut exit: EventWriter<AppExit>, mut robots: Query<&mut Robot>) {
    let mut robot = robots.iter_mut().next().unwrap();

    robot.database.main_outputs.ground_to_field = Some(Isometry2::identity());

    let angle = 0.01 * time.ticks() as f32;
    let (sin, cos) = angle.sin_cos();
    let k = (angle / TAU) as usize;

    let (path, orientation_mode, target_orientation) = match k {
        0 => {
            let target_point = point![cos, sin];

            (
                Path {
                    segments: vec![PathSegment::LineSegment(
                        geometry::line_segment::LineSegment(Point2::origin(), target_point),
                    )],
                },
                OrientationMode::Unspecified,
                Orientation2::identity(),
            )
        }
        1 => (
            Path {
                segments: vec![PathSegment::LineSegment(
                    geometry::line_segment::LineSegment(Point2::origin(), point![1.0, 0.0]),
                )],
            },
            OrientationMode::LookTowards(Orientation2::new(angle)),
            Orientation2::new(angle),
        ),
        _ => {
            exit.send(AppExit::Success);
            return;
        }
    };

    robot.parameters.behavior.injected_motion_command = Some(MotionCommand::Walk {
        path,
        orientation_mode,
        target_orientation,
        distance_to_be_aligned: 0.1,
        head: HeadMotion::ZeroAngles,
        left_arm: ArmMotion::Swing,
        right_arm: ArmMotion::Swing,
        speed: WalkSpeed::Normal,
    });
}
