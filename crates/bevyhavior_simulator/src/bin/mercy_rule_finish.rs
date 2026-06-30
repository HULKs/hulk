use std::time::{Duration, SystemTime};

use bevy::{ecs::system::SystemParam, prelude::*};
use bevyhavior_simulator::behavior_tree_simulator::{
    AutoRefereeConfig, BehaviorTreeSimulatorSet, SimulatedBall, SimulatorBall, SimulatorClock,
    SimulatorGameState, SimulatorRobotBundle, SimulatorTimelineMarkers,
    default_behavior_parameters,
};
use eframe::egui::Color32;
use hsl_network_messages::{PlayerNumber, Team};
use linear_algebra::{Isometry2, point, vector};
use scenario::scenario;
use types::{field_dimensions::Side, primary_state::PrimaryState};

#[scenario]
fn mercy_rule_finish(app: &mut App) {
    app.add_systems(Startup, startup)
        .add_systems(Update, update.in_set(BehaviorTreeSimulatorSet::Scenario));
}

fn startup(mut commands: Commands, mut ball: ResMut<SimulatorBall>) {
    let mut parameters =
        default_behavior_parameters().expect("failed to load default behavior parameters");
    parameters.goal_keeper_number = PlayerNumber::One;
    parameters.last_ball_timeout = Duration::from_secs(2);

    for (player_number, pose) in [
        (
            PlayerNumber::Three,
            Isometry2::from_parts(vector![-0.4, 0.0], 0.0),
        ),
        (
            PlayerNumber::Four,
            Isometry2::from_parts(vector![-1.0, 1.0], 0.0),
        ),
        (
            PlayerNumber::Five,
            Isometry2::from_parts(vector![-1.0, -1.0], 0.0),
        ),
    ] {
        commands.spawn(
            SimulatorRobotBundle::new(Team::Hulks, player_number, pose, parameters.clone())
                .expect("failed to create robot bundle")
                .with_primary_state(PrimaryState::Playing),
        );
    }

    ball.state = Some(SimulatedBall {
        position: point![1.0, 0.0],
        velocity: vector![0.0, 0.0],
        field_side: Side::Left,
    });
}

#[derive(SystemParam)]
struct LastScore<'s> {
    hulks: Local<'s, u8>,
    opponent: Local<'s, u8>,
}

fn update(
    clock: Res<SimulatorClock>,
    auto_referee_config: Res<AutoRefereeConfig>,
    game_state: Res<SimulatorGameState>,
    mut last_score: LastScore,
    mut timeline_markers: ResMut<SimulatorTimelineMarkers>,
    mut exit: MessageWriter<AppExit>,
) {
    let hulks_score = game_state.game_controller_state.hulks_team.score;
    let opponent_score = game_state.game_controller_state.opponent_team.score;
    let elapsed = clock
        .now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("simulator time should not move backwards");

    if hulks_score > *last_score.hulks {
        timeline_markers.add(
            clock.now,
            Color32::LIGHT_GREEN,
            format!(
                "{:.2}s HULKs goal {hulks_score}:{opponent_score}",
                elapsed.as_secs_f32()
            ),
        );
        *last_score.hulks = hulks_score;
    }
    if opponent_score > *last_score.opponent {
        timeline_markers.add(
            clock.now,
            Color32::LIGHT_RED,
            format!(
                "{:.2}s opponent goal {hulks_score}:{opponent_score}",
                elapsed.as_secs_f32(),
            ),
        );
        *last_score.opponent = opponent_score;
    }

    let goal_difference = hulks_score.abs_diff(opponent_score);
    if goal_difference >= 10 {
        println!(
            "ok {:.2}s score: {hulks_score}:{opponent_score}",
            elapsed.as_secs_f32()
        );
        exit.write(AppExit::Success);
    }

    if elapsed > auto_referee_config.halftime_duration {
        println!(
            "failed: no mercy rule within one halftime
             {:.2}s score: {hulks_score}:{opponent_score}",
            elapsed.as_secs_f32()
        );
        exit.write(AppExit::from_code(2));
    }
}
