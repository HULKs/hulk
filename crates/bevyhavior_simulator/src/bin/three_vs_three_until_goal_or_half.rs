use std::{
    f32::consts::PI,
    time::{Duration, SystemTime},
};

use bevy::prelude::*;
use bevyhavior_simulator::behavior_tree_simulator::{
    AutoRefereeConfig, BehaviorTreeSimulatorSet, SimulatedBall, SimulatorBall, SimulatorClock,
    SimulatorGameState, SimulatorRobotBundle, default_behavior_parameters,
};
use coordinate_systems::{Ground, World};
use hsl_network_messages::{GameState, PlayerNumber, Team};
use linear_algebra::{Isometry2, point, vector};
use scenario::scenario;
use types::{field_dimensions::Side, primary_state::PrimaryState};

#[scenario]
fn three_vs_three_until_goal_or_half(app: &mut App) {
    app.add_systems(Startup, startup)
        .add_systems(Update, update.in_set(BehaviorTreeSimulatorSet::Scenario));
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
        (Team::Opponent, PlayerNumber::Three, pose(0.8, 0.0, PI)),
        (Team::Opponent, PlayerNumber::Four, pose(1.5, -1.0, PI)),
        (Team::Opponent, PlayerNumber::Five, pose(1.5, 1.0, PI)),
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

fn update(
    clock: Res<SimulatorClock>,
    auto_referee_config: Res<AutoRefereeConfig>,
    game_state: Res<SimulatorGameState>,
    mut exit: MessageWriter<AppExit>,
) {
    let hulks_score = game_state.game_controller_state.hulks_team.score;
    let opponent_score = game_state.game_controller_state.opponent_team.score;
    let elapsed = clock
        .now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("simulator time should not move backwards");
    if hulks_score > 0 || opponent_score > 0 {
        println!(
            "result=goal elapsed={:.2} hulks_score={hulks_score} opponent_score={opponent_score}",
            elapsed.as_secs_f32()
        );
        exit.write(AppExit::Success);
        return;
    }

    if game_state.game_controller_state.game_state == GameState::Finished {
        println!(
            "result=halftime elapsed={:.2} hulks_score={hulks_score} opponent_score={opponent_score}",
            elapsed.as_secs_f32()
        );
        exit.write(AppExit::Success);
        return;
    }

    if elapsed > auto_referee_config.halftime_duration + Duration::from_secs(1) {
        println!(
            "result=fail elapsed={:.2} hulks_score={hulks_score} opponent_score={opponent_score} reason=halftime_timeout_not_reached",
            elapsed.as_secs_f32()
        );
        exit.write(AppExit::from_code(1));
    }
}

fn pose(x: f32, y: f32, yaw: f32) -> Isometry2<Ground, World> {
    Isometry2::from_parts(vector![x, y], yaw)
}
