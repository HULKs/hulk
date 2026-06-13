use std::time::{Duration, SystemTime};

use bevy::prelude::*;
use bevyhavior_simulator::behavior_tree_simulator::{
    AutoRefereeConfig, BehaviorTreeSimulatorSet, SimulatorBall, SimulatorClock, SimulatorGameState,
    SimulatorRobotBundle, default_behavior_parameters,
};
use coordinate_systems::{Ground, World};
use hsl_network_messages::{GameState, PlayerNumber};
use linear_algebra::{Isometry2, point, vector};
use scenario::scenario;
use types::primary_state::PrimaryState;

#[derive(Resource, Default)]
struct LastScore {
    hulks: u8,
    opponent: u8,
}

#[scenario]
fn mercy_rule_finish(app: &mut App) {
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

    for (player_number, pose) in [
        (PlayerNumber::Three, kickoff_pose(PlayerNumber::Three)),
        (PlayerNumber::Four, kickoff_pose(PlayerNumber::Four)),
        (PlayerNumber::Five, kickoff_pose(PlayerNumber::Five)),
    ] {
        commands.spawn(
            SimulatorRobotBundle::new(player_number, pose, parameters.clone())
                .expect("failed to create robot bundle")
                .with_primary_state(PrimaryState::Playing),
        );
    }

    ball.state = Some(
        bevyhavior_simulator::behavior_tree_simulator::SimulatedBall {
            position: point![1.0, 0.0],
            velocity: vector![0.0, 0.0],
            field_side: types::field_dimensions::Side::Left,
        },
    );
}

fn report_and_exit(
    clock: Res<SimulatorClock>,
    auto_referee_config: Res<AutoRefereeConfig>,
    game_state: Res<SimulatorGameState>,
    mut last_score: ResMut<LastScore>,
    mut exit: MessageWriter<AppExit>,
) {
    let hulks_score = game_state.game_controller_state.hulks_team.score;
    let opponent_score = game_state.game_controller_state.opponent_team.score;
    if hulks_score != last_score.hulks || opponent_score != last_score.opponent {
        println!(
            "score hulks={hulks_score} opponent={opponent_score} elapsed={:.2}",
            elapsed(clock.now).as_secs_f32()
        );
        last_score.hulks = hulks_score;
        last_score.opponent = opponent_score;
    }

    let elapsed = elapsed(clock.now);
    let goal_difference = hulks_score.abs_diff(opponent_score);
    if game_state.game_controller_state.game_state == GameState::Finished {
        if goal_difference >= 10 && elapsed <= auto_referee_config.halftime_duration {
            println!(
                "result=ok elapsed={:.2} hulks_score={hulks_score} opponent_score={opponent_score}",
                elapsed.as_secs_f32()
            );
            exit.write(AppExit::Success);
        } else {
            println!(
                "result=fail elapsed={:.2} hulks_score={hulks_score} opponent_score={opponent_score} reason=no_mercy_within_halftime",
                elapsed.as_secs_f32()
            );
            exit.write(AppExit::from_code(1));
        }
        return;
    }

    if elapsed > auto_referee_config.halftime_duration + Duration::from_secs(1) {
        println!(
            "result=fail elapsed={:.2} hulks_score={hulks_score} opponent_score={opponent_score} reason=timeout",
            elapsed.as_secs_f32()
        );
        exit.write(AppExit::from_code(2));
    }
}

fn elapsed(now: SystemTime) -> Duration {
    now.duration_since(SystemTime::UNIX_EPOCH)
        .expect("simulator time should not move backwards")
}

fn pose(x: f32, y: f32, yaw: f32) -> Isometry2<Ground, World> {
    Isometry2::from_parts(vector![x, y], yaw)
}

fn kickoff_pose(player_number: PlayerNumber) -> Isometry2<Ground, World> {
    match player_number {
        PlayerNumber::Three => pose(-0.4, 0.0, 0.0),
        PlayerNumber::Four => pose(-1.0, 1.0, 0.0),
        PlayerNumber::Five => pose(-1.0, -1.0, 0.0),
        _ => pose(-1.5, 0.0, 0.0),
    }
}
