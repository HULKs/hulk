use std::time::{Duration, SystemTime};

use bevy::prelude::*;
use bevyhavior_simulator::behavior_tree_simulator::{
    AutoRefereeConfig, BehaviorTreeSimulatorSet, SimulatedBall, SimulatorBall, SimulatorClock,
    SimulatorGameState, SimulatorRobotBundle, SimulatorTimelineMarkers,
    default_behavior_parameters,
};
use coordinate_systems::{Ground, World};
use eframe::egui::Color32;
use hsl_network_messages::{GameState, PlayerNumber, Team};
use linear_algebra::{Isometry2, point, vector};
use scenario::scenario;
use types::{field_dimensions::Side, primary_state::PrimaryState};

#[derive(Resource, Default)]
struct LastScore {
    hulks: u8,
    opponent: u8,
}

#[scenario]
fn three_vs_three_until_goal_or_half(app: &mut App) {
    app.init_resource::<LastScore>()
        .add_systems(Startup, startup)
        .add_systems(
            Update,
            report_and_exit.in_set(BehaviorTreeSimulatorSet::Scenario),
        );
}

fn startup(mut commands: Commands, mut ball: ResMut<SimulatorBall>) {
    let mut parameters =
        default_behavior_parameters().expect("failed to load default behavior parameters");
    parameters.goal_keeper_number = PlayerNumber::One;
    parameters.last_ball_timeout = Duration::from_secs(2);

    for (team, player_number, pose) in [
        (Team::Hulks, PlayerNumber::Three, pose(-0.8, 0.0, 0.0)),
        (Team::Hulks, PlayerNumber::Four, pose(-1.5, 1.0, 0.0)),
        (Team::Hulks, PlayerNumber::Five, pose(-1.5, -1.0, 0.0)),
        (
            Team::Opponent,
            PlayerNumber::Three,
            pose(0.8, 0.0, std::f32::consts::PI),
        ),
        (
            Team::Opponent,
            PlayerNumber::Four,
            pose(1.5, -1.0, std::f32::consts::PI),
        ),
        (
            Team::Opponent,
            PlayerNumber::Five,
            pose(1.5, 1.0, std::f32::consts::PI),
        ),
    ] {
        commands.spawn(
            SimulatorRobotBundle::new(team, player_number, pose, parameters.clone())
                .expect("failed to create robot bundle")
                .with_primary_state(PrimaryState::Playing),
        );
    }

    ball.state = Some(SimulatedBall {
        position: point![0.0, 0.0],
        velocity: vector![0.0, 0.0],
        field_side: Side::Left,
    });
}

fn report_and_exit(
    clock: Res<SimulatorClock>,
    auto_referee_config: Res<AutoRefereeConfig>,
    game_state: Res<SimulatorGameState>,
    mut last_score: ResMut<LastScore>,
    mut timeline_markers: ResMut<SimulatorTimelineMarkers>,
    mut exit: MessageWriter<AppExit>,
) {
    let hulks_score = game_state.game_controller_state.hulks_team.score;
    let opponent_score = game_state.game_controller_state.opponent_team.score;
    if hulks_score != last_score.hulks || opponent_score != last_score.opponent {
        add_goal_markers(
            clock.now,
            hulks_score,
            opponent_score,
            &last_score,
            &mut timeline_markers,
        );
        println!(
            "result=goal elapsed={:.2} hulks_score={hulks_score} opponent_score={opponent_score}",
            elapsed(clock.now).as_secs_f32()
        );
        last_score.hulks = hulks_score;
        last_score.opponent = opponent_score;
        exit.write(AppExit::Success);
        return;
    }

    if game_state.game_controller_state.game_state == GameState::Finished {
        println!(
            "result=halftime elapsed={:.2} hulks_score={hulks_score} opponent_score={opponent_score}",
            elapsed(clock.now).as_secs_f32()
        );
        exit.write(AppExit::Success);
        return;
    }

    if elapsed(clock.now) > auto_referee_config.halftime_duration + Duration::from_secs(1) {
        println!(
            "result=fail elapsed={:.2} hulks_score={hulks_score} opponent_score={opponent_score} reason=halftime_timeout_not_reached",
            elapsed(clock.now).as_secs_f32()
        );
        exit.write(AppExit::from_code(1));
    }
}

fn add_goal_markers(
    now: SystemTime,
    hulks_score: u8,
    opponent_score: u8,
    last_score: &LastScore,
    timeline_markers: &mut SimulatorTimelineMarkers,
) {
    for score in (last_score.hulks + 1)..=hulks_score {
        timeline_markers.add(
            now,
            Color32::LIGHT_GREEN,
            format!("HULKs goal {score}:{opponent_score}"),
        );
    }
    for score in (last_score.opponent + 1)..=opponent_score {
        timeline_markers.add(
            now,
            Color32::LIGHT_RED,
            format!("opponent goal {hulks_score}:{score}"),
        );
    }
}

fn elapsed(now: SystemTime) -> Duration {
    now.duration_since(SystemTime::UNIX_EPOCH)
        .expect("simulator time should not move backwards")
}

fn pose(x: f32, y: f32, yaw: f32) -> Isometry2<Ground, World> {
    Isometry2::from_parts(vector![x, y], yaw)
}
