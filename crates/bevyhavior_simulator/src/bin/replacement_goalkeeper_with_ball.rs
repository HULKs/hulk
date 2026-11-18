//! Tests replacement goalkeeper election with a known ball.
//! Initially, player two becomes the replacement while player three
//! is responsible for the ball. The ball is then moved toward player two.
//! Verifies that the replacement role and claim remain stable while ball
//! responsibility changes. Robot positions remain fixed, and both robots
//! can perceive the ball throughout the scenario.

use std::time::{Duration, SystemTime};

use bevy::prelude::*;
use bevyhavior_simulator::behavior_tree_simulator::{
    BehaviorTreeSimulatorSet, SimulatedBall, SimulationConfig, SimulatorBall, SimulatorClock,
    SimulatorFieldDimensions, SimulatorGameState, SimulatorRobot, SimulatorRobotBehavior,
    SimulatorRobotBundle, default_behavior_parameters,
};
use hsl_network_messages::{Penalty, PlayerNumber, Team};
use linear_algebra::{Isometry2, point, vector};
use scenario::scenario;
use types::{field_dimensions::Side, primary_state::PrimaryState};

#[scenario]
fn replacement_goalkeeper_with_ball(app: &mut App) {
    app.add_systems(Startup, startup).add_systems(
        Update,
        check_result.in_set(BehaviorTreeSimulatorSet::Scenario),
    );
}

fn startup(
    mut commands: Commands,
    mut ball: ResMut<SimulatorBall>,
    mut game_state: ResMut<SimulatorGameState>,
    mut config: ResMut<SimulationConfig>,
    field_dimensions: Res<SimulatorFieldDimensions>,
) {
    // Keep the geometry stable while testing role selection.
    config.walk_translation_speed = 0.0;
    config.walk_rotation_speed = 0.0;
    config.walk_with_velocity_scale = 0.0;

    // Keep the ball visible to both robots after repositioning it.
    config.visibility_field_of_view = std::f32::consts::TAU;

    ball.state = Some(SimulatedBall {
        position: point![0.0, 0.0],
        velocity: vector![0.0, 0.0],
        field_side: Side::Left,
    });

    game_state.game_controller_state.penalties.one = Some(Penalty::PickUp {
        remaining: Duration::from_secs(30),
    });
    game_state.sync_filtered_game_controller_state();

    let mut parameters = default_behavior_parameters().expect("failed to load behavior parameters");
    parameters.goalkeeper.player_number = PlayerNumber::One;
    parameters.control.is_simple = false;

    for (player_number, position) in [
        (PlayerNumber::Two, vector![-3.0, 1.0]),
        (PlayerNumber::Three, vector![-1.0, 0.0]),
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
    mut ball: ResMut<SimulatorBall>,
    mut ball_has_moved: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    let elapsed = clock
        .now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("simulator time must not move backwards");

    if elapsed < Duration::from_secs(3) {
        return;
    }

    let mut active_replacements = Vec::new();
    let mut claiming_players = Vec::new();
    let mut ball_responsible_players = Vec::new();
    let mut players_with_known_ball = 0;
    let mut player_three_received_claim = false;

    for (robot, behavior) in &robots {
        if robot.team != Team::Hulks {
            continue;
        }

        let blackboard = &behavior.blackboard;
        let coordinator = &blackboard.replacement_goalkeeper_coordinator;

        if coordinator.is_active() {
            active_replacements.push(robot.player_number);
        }
        if coordinator.claims_role() {
            claiming_players.push(robot.player_number);
        }
        if blackboard.world_state.ball.is_some() {
            players_with_known_ball += 1;
        }
        if blackboard.last_closest_to_ball {
            ball_responsible_players.push(robot.player_number);
        }

        if robot.player_number == PlayerNumber::Three {
            player_three_received_claim = blackboard
                .world_state
                .player_states
                .two
                .as_ref()
                .is_some_and(|state| state.replacement_goalkeeper_claim);
        }
    }

    // Check role stability on every tick after the initial election.
    let replacement_role_is_stable = active_replacements == vec![PlayerNumber::Two]
        && claiming_players == vec![PlayerNumber::Two]
        && player_three_received_claim;

    if !replacement_role_is_stable {
        println!(
            "unstable role: active={active_replacements:?}, \
             claiming={claiming_players:?}, \
             received_claim={player_three_received_claim}"
        );
        exit.write(AppExit::from_code(1));
        return;
    }

    if !*ball_has_moved {
        let initial_state_is_correct =
            ball_responsible_players == vec![PlayerNumber::Three] && players_with_known_ball == 2;

        println!(
            "initial: active={active_replacements:?}, \
             ball_responsible={ball_responsible_players:?}, \
             known_ball={players_with_known_ball}"
        );

        if !initial_state_is_correct {
            exit.write(AppExit::from_code(1));
            return;
        }

        // Player two is at (-3, 1), player three at (-1, 0).
        ball.state = Some(SimulatedBall {
            position: point![-2.5, 1.0],
            velocity: vector![0.0, 0.0],
            field_side: Side::Left,
        });
        *ball_has_moved = true;
        return;
    }

    // Allow perception and hysteresis to react to the new ball position.
    if elapsed < Duration::from_secs(4) {
        return;
    }

    let success =
        ball_responsible_players == vec![PlayerNumber::Two] && players_with_known_ball == 2;

    println!(
        "ball_moved: active={active_replacements:?}, \
         claiming={claiming_players:?}, \
         ball_responsible={ball_responsible_players:?}, \
         known_ball={players_with_known_ball}, \
         received_claim={player_three_received_claim}"
    );

    exit.write(if success {
        AppExit::Success
    } else {
        AppExit::from_code(1)
    });
}
