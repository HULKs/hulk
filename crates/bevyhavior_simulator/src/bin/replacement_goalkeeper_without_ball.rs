//! Tests replacement goalkeeper election without a known ball.
//! Player one is penalized; player two is closest to the own goal.
//! Verifies that only player two becomes active and player three receives
//! its claim. After the penalty is cleared, verifies that the replacement
//! role ends and player three receives the withdrawn claim.

use std::time::{Duration, SystemTime};

use bevy::prelude::*;
use bevyhavior_simulator::behavior_tree_simulator::{
    BehaviorTreeSimulatorSet, SimulatorBall, SimulatorClock, SimulatorFieldDimensions,
    SimulatorGameState, SimulatorRobot, SimulatorRobotBehavior, SimulatorRobotBundle,
    default_behavior_parameters,
};
use hsl_network_messages::{Penalty, PlayerNumber, Team};
use linear_algebra::{Isometry2, vector};
use scenario::scenario;
use types::primary_state::PrimaryState;

#[scenario]
fn replacement_goalkeeper_without_ball(app: &mut App) {
    app.add_systems(Startup, startup).add_systems(
        Update,
        check_result.in_set(BehaviorTreeSimulatorSet::Scenario),
    );
}

fn startup(
    mut commands: Commands,
    mut ball: ResMut<SimulatorBall>,
    mut game_state: ResMut<SimulatorGameState>,
    field_dimensions: Res<SimulatorFieldDimensions>,
) {
    ball.state = None;

    game_state.game_controller_state.penalties.one = Some(Penalty::PickUp {
        remaining: Duration::from_secs(30),
    });
    game_state.sync_filtered_game_controller_state();

    let mut parameters = default_behavior_parameters().expect("failed to load behavior parameters");
    parameters.goalkeeper.player_number = PlayerNumber::One;
    parameters.control.is_simple = false;

    for (player_number, position) in [
        (PlayerNumber::Two, vector![-3.0, 0.0]),
        (PlayerNumber::Three, vector![-1.5, 1.0]),
    ] {
        commands.spawn(
            SimulatorRobotBundle::new(
                Team::Hulks,
                player_number,
                Isometry2::from_parts(position, 0.0),
                field_dimensions.0,
                parameters.clone(),
            )
            .expect("failed to create robot bundle")
            .with_primary_state(PrimaryState::Playing),
        );
    }
}

fn check_result(
    clock: Res<SimulatorClock>,
    robots: Query<(&SimulatorRobot, &SimulatorRobotBehavior)>,
    mut game_state: ResMut<SimulatorGameState>,
    mut regular_goalkeeper_returned: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    let elapsed = clock
        .now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("simulator time must not move backwards");

    let check_time = if *regular_goalkeeper_returned {
        Duration::from_secs(4)
    } else {
        Duration::from_secs(3)
    };

    if elapsed < check_time {
        return;
    }

    let mut active_replacements = Vec::new();
    let mut claiming_players = Vec::new();
    let mut player_three_received_claim = None;

    for (robot, behavior) in &robots {
        if robot.team != Team::Hulks {
            continue;
        }

        let coordinator = &behavior.blackboard.replacement_goalkeeper_coordinator;

        if coordinator.is_active() {
            active_replacements.push(robot.player_number);
        }
        if coordinator.claims_role() {
            claiming_players.push(robot.player_number);
        }

        if robot.player_number == PlayerNumber::Three {
            player_three_received_claim = behavior
                .blackboard
                .world_state
                .player_states
                .two
                .as_ref()
                .map(|state| state.replacement_goalkeeper_claim);
        }
    }

    if !*regular_goalkeeper_returned {
        let election_succeeded = active_replacements == vec![PlayerNumber::Two]
            && claiming_players == vec![PlayerNumber::Two]
            && player_three_received_claim == Some(true);

        println!(
            "election: active={active_replacements:?}, \
             claiming={claiming_players:?}, \
             received_claim={player_three_received_claim:?}"
        );

        if !election_succeeded {
            exit.write(AppExit::from_code(1));
            return;
        }

        // Simulate the regular goalkeeper's return through the GC state.
        game_state.game_controller_state.penalties.one = None;
        game_state.sync_filtered_game_controller_state();
        *regular_goalkeeper_returned = true;
        return;
    }

    let success = active_replacements.is_empty()
        && claiming_players.is_empty()
        && player_three_received_claim == Some(false);

    println!(
        "keeper_returned: active={active_replacements:?}, \
         claiming={claiming_players:?}, \
         received_claim={player_three_received_claim:?}"
    );

    exit.write(if success {
        AppExit::Success
    } else {
        AppExit::from_code(1)
    });
}
