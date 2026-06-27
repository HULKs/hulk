use std::time::Duration;

use bevy::prelude::*;
use bevyhavior_simulator::behavior_tree_simulator::{
    BehaviorTreeSimulatorSet, SimulatorBall, SimulatorGameState, SimulatorObstacle,
    SimulatorRobotBundle, SimulatorScenarioObstacles, SimulatorTimeline,
    default_behavior_parameters,
};
use coordinate_systems::{Ground, World};
use hsl_network_messages::{PlayerNumber, Team};
use linear_algebra::{Isometry2, point, vector};
use scenario::scenario;
use types::{motion_command::MotionCommand, primary_state::PrimaryState};

const RUN_DURATION: Duration = Duration::from_secs(20);

#[derive(Resource, Default)]
struct PrintedFrames(usize);

#[scenario]
fn behavior_tree_smoke(app: &mut App) {
    app.init_resource::<PrintedFrames>()
        .add_systems(Startup, startup)
        .add_systems(
            Update,
            report_and_exit.in_set(BehaviorTreeSimulatorSet::Scenario),
        );
}

fn startup(
    mut commands: Commands,
    mut ball: ResMut<SimulatorBall>,
    mut scenario_obstacles: ResMut<SimulatorScenarioObstacles>,
) {
    let mut parameters =
        default_behavior_parameters().expect("failed to load default behavior parameters");
    parameters.goal_keeper_number = PlayerNumber::One;
    parameters.last_ball_timeout = Duration::from_secs(2);

    commands.spawn(
        SimulatorRobotBundle::new(
            Team::Hulks,
            PlayerNumber::Three,
            pose(0.0, 0.0, 0.0),
            parameters.clone(),
        )
        .expect("failed to create robot bundle")
        .with_primary_state(PrimaryState::Playing),
    );
    commands.spawn(
        SimulatorRobotBundle::new(
            Team::Hulks,
            PlayerNumber::Four,
            pose(-1.0, 1.0, 0.0),
            parameters,
        )
        .expect("failed to create robot bundle")
        .with_primary_state(PrimaryState::Playing),
    );
    scenario_obstacles.add(SimulatorObstacle::robot(point![2.0, -0.1], 0.3, 0.5));

    ball.state = Some(
        bevyhavior_simulator::behavior_tree_simulator::SimulatedBall {
            position: point![1.0, 0.0],
            velocity: vector![0.0, 0.0],
            field_side: types::field_dimensions::Side::Left,
        },
    );
}

fn report_and_exit(
    timeline: Res<SimulatorTimeline>,
    game_state: Res<SimulatorGameState>,
    mut printed_frames: ResMut<PrintedFrames>,
    mut exit: MessageWriter<AppExit>,
) {
    for (index, frame) in timeline.frames.iter().enumerate().skip(printed_frames.0) {
        println!(
            "frame={index} violations={}",
            frame.invariant_violations.len()
        );
        for (player_number, robot_frame) in &frame.robot_frames {
            println!(
                "  robot={player_number} motion={}",
                motion_name(&robot_frame.motion_command)
            );
        }
        for violation in &frame.invariant_violations {
            println!(
                "  invariant={} robot={:?} severity={:?} message={}",
                violation.check_name,
                violation.player_number,
                violation.severity,
                violation.message
            );
        }
    }
    printed_frames.0 = timeline.frames.len();
    let score = game_state.game_controller_state.hulks_team.score
        + game_state.game_controller_state.opponent_team.score;
    if score > 0 {
        println!(
            "result=ok frames={} hulks_score={} opponent_score={}",
            timeline.frames.len(),
            game_state.game_controller_state.hulks_team.score,
            game_state.game_controller_state.opponent_team.score,
        );
        exit.write(AppExit::Success);
        return;
    }

    if timeline.frames.len() as u32 >= frames_to_run() {
        println!(
            "result=fail frames={} reason=no_goal",
            timeline.frames.len()
        );
        exit.write(AppExit::from_code(1));
    }
}

fn frames_to_run() -> u32 {
    (RUN_DURATION.as_secs_f32()
        / bevyhavior_simulator::behavior_tree_simulator::DEFAULT_TICK_DURATION.as_secs_f32())
    .ceil() as u32
}

fn pose(x: f32, y: f32, yaw: f32) -> Isometry2<Ground, World> {
    Isometry2::from_parts(vector![x, y], yaw)
}

fn motion_name(motion_command: &MotionCommand) -> &'static str {
    match motion_command {
        MotionCommand::Damping => "damping",
        MotionCommand::Prepare => "prepare",
        MotionCommand::Stand { .. } => "stand",
        MotionCommand::StandUp => "stand_up",
        MotionCommand::VisualKick { .. } => "visual_kick",
        MotionCommand::Walk { .. } => "walk",
        MotionCommand::WalkWithVelocity { .. } => "walk_with_velocity",
    }
}
