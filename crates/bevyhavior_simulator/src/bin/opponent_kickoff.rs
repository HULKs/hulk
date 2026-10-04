//! Regression for approaching the ball during the opponent's restricted kickoff.
//! Run with `cargo run -p bevyhavior_simulator --bin opponent_kickoff` for the viewer,
//! or `cargo test -p bevyhavior_simulator --bin opponent_kickoff` headlessly.
use std::time::{Duration, SystemTime};

use bevy::prelude::*;
use bevyhavior_simulator::behavior_tree_simulator::{
    BehaviorTreeSimulatorSet, SimulatedBall, SimulatorAutoReferee, SimulatorBall, SimulatorClock,
    SimulatorFieldDimensions, SimulatorGameState, SimulatorRobotBundle, SimulatorRobotFrames,
    SimulatorRobotId, SimulatorTimelineMarkers, SimulatorWorldStates, default_behavior_parameters,
};
use coordinate_systems::Field;
use eframe::egui::Color32;
use geometry::{circle::Circle, rectangle::Rectangle};
use hsl_network_messages::{GameState, PlayerNumber, Team};
use linear_algebra::{Isometry2, Point2, point, vector};
use scenario::scenario;
use types::{
    field_dimensions::Side, filtered_game_state::FilteredGameState, motion_command::MotionCommand,
    primary_state::PrimaryState, rule_obstacles::RuleObstacle,
};

const PLAYER: PlayerNumber = PlayerNumber::Two;
const SET_DURATION: Duration = Duration::from_secs(3);
const RESTRICTED_DURATION: Duration = Duration::from_secs(10);
const FAILURE_TAIL_DURATION: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Ready,
    Set,
    Restricted,
    Released,
}

#[derive(Resource)]
struct KickoffScenario {
    phase: Phase,
    since: SystemTime,
    ready_position: Point2<Field>,
    waiting_position: Option<Point2<Field>>,
    walked_in_ready: bool,
    failed_at: Option<SystemTime>,
}

/// AI generated scenario to test that the robot does not kick the ball when the
/// opponent has kickoff.
#[scenario]
fn opponent_kickoff(app: &mut App) {
    app.add_systems(Startup, startup)
        .add_systems(
            Update,
            supply_kickoff_restriction.in_set(BehaviorTreeSimulatorSet::AfterWorldState),
        )
        .add_systems(
            Update,
            check_and_advance.in_set(BehaviorTreeSimulatorSet::Scenario),
        );
}

fn startup(
    mut commands: Commands,
    clock: Res<SimulatorClock>,
    mut referee: ResMut<SimulatorAutoReferee>,
    mut game: ResMut<SimulatorGameState>,
    mut ball: ResMut<SimulatorBall>,
    mut markers: ResMut<SimulatorTimelineMarkers>,
) {
    // This scenario owns transitions: Ready ends only after the robot arrives.
    referee.rules.clear();
    game.set_kicking_team(Some(Team::Opponent));
    game.set_game_state(GameState::Ready, clock.now);
    let parameters = default_behavior_parameters().expect("failed to load behavior parameters");
    let ready_position = parameters.kickoff.standard_positions[PLAYER].position;
    commands.insert_resource(KickoffScenario {
        phase: Phase::Ready,
        since: clock.now,
        ready_position,
        waiting_position: None,
        walked_in_ready: false,
        failed_at: None,
    });
    commands.spawn(
        SimulatorRobotBundle::new(
            Team::Hulks,
            PLAYER,
            Isometry2::from_parts(vector![-4.0, -2.0], 0.0),
            parameters,
        )
        .expect("failed to create robot")
        .with_primary_state(PrimaryState::Ready),
    );
    // The opponent owns kickoff but does not take it; the ball stays at center.
    ball.state = Some(SimulatedBall {
        position: point![0.0, 0.0],
        velocity: vector![0.0, 0.0],
        field_side: Side::Left,
    });
    markers.add(clock.now, Color32::YELLOW, "Opponent kickoff: Ready");
}

fn supply_kickoff_restriction(
    scenario: Res<KickoffScenario>,
    dimensions: Res<SimulatorFieldDimensions>,
    mut worlds: ResMut<SimulatorWorldStates>,
) {
    if !matches!(scenario.phase, Phase::Restricted | Phase::Released) {
        return;
    }
    // The simulator's simplified GC conversion always frees the ball in Playing.
    // Supply the filtered state seen in recover2.mcap before behavior ticks.
    let world = worlds
        .0
        .get_mut(&SimulatorRobotId::new(Team::Hulks, PLAYER))
        .expect("scenario robot must have a world state");
    let filtered = world
        .filtered_game_controller_state
        .as_mut()
        .expect("scenario must have a game controller");
    filtered.game_state = FilteredGameState::Playing {
        ball_is_free: scenario.phase == Phase::Released,
        kick_off: true,
    };
    filtered.opponent_game_state = FilteredGameState::Playing {
        ball_is_free: true,
        kick_off: true,
    };
    if scenario.phase == Phase::Restricted {
        // Include the center-circle and opponent-half obstacles present in the log.
        world.rule_obstacles = vec![
            RuleObstacle::Circle(Circle::new(
                Point2::origin(),
                dimensions.0.center_circle_diameter / 2.0 + 0.2,
            )),
            RuleObstacle::Rectangle(Rectangle {
                min: point![0.0, -dimensions.0.width / 2.0],
                max: point![dimensions.0.length / 2.0, dimensions.0.width / 2.0],
            }),
        ];
    }
}

fn check_and_advance(
    clock: Res<SimulatorClock>,
    frames: Res<SimulatorRobotFrames>,
    ball: Res<SimulatorBall>,
    mut scenario: ResMut<KickoffScenario>,
    mut game: ResMut<SimulatorGameState>,
    mut markers: ResMut<SimulatorTimelineMarkers>,
    mut exit: MessageWriter<AppExit>,
) {
    // Keep behavior and physics running in the phase that failed for the viewer.
    if let Some(failed_at) = scenario.failed_at {
        if clock
            .now
            .duration_since(failed_at)
            .expect("monotonic simulation time")
            >= FAILURE_TAIL_DURATION
        {
            exit.write(AppExit::from_code(1));
        }
        return;
    }
    let frame = frames
        .0
        .get(&SimulatorRobotId::new(Team::Hulks, PLAYER))
        .expect("scenario robot must produce a behavior frame");
    let pose = frame
        .world_state
        .robot
        .ground_to_field
        .expect("robot must be localized");
    let position = pose.translation();
    let elapsed = clock
        .now
        .duration_since(scenario.since)
        .expect("monotonic simulation time");
    let is_standing = matches!(frame.motion_command, MotionCommand::Stand { .. });

    let failure = match scenario.phase {
        Phase::Ready => {
            scenario.walked_in_ready |= matches!(frame.motion_command, MotionCommand::Walk { .. });
            if is_standing && (position - scenario.ready_position).norm() < 0.2 {
                if !scenario.walked_in_ready {
                    Some("robot never walked to its Ready position")
                } else {
                    scenario.waiting_position = Some(position);
                    scenario.phase = Phase::Set;
                    scenario.since = clock.now;
                    game.set_game_state(GameState::Set, clock.now);
                    markers.add(clock.now, Color32::YELLOW, "Ready position reached: Set");
                    None
                }
            } else if elapsed > Duration::from_secs(45) {
                Some("robot did not reach its Ready position within 45 seconds")
            } else {
                None
            }
        }
        Phase::Set | Phase::Restricted => {
            let waiting_position = scenario.waiting_position.expect("Ready position recorded");
            if !is_standing {
                Some("robot commanded motion during Set or restricted opponent kickoff")
            } else if (position - waiting_position).norm() > 0.01 {
                Some("robot moved from its waiting position")
            } else if ball
                .state
                .is_none_or(|ball| ball.position.coords().norm() > 0.001)
            {
                Some("ball moved before kickoff was released")
            } else if scenario.phase == Phase::Set && elapsed >= SET_DURATION {
                scenario.phase = Phase::Restricted;
                scenario.since = clock.now;
                game.set_game_state(GameState::Playing, clock.now);
                markers.add(clock.now, Color32::LIGHT_RED, "Playing: ball is NOT free");
                None
            } else if scenario.phase == Phase::Restricted && elapsed >= RESTRICTED_DURATION {
                scenario.phase = Phase::Released;
                scenario.since = clock.now;
                markers.add(clock.now, Color32::LIGHT_GREEN, "Playing: ball is free");
                None
            } else {
                None
            }
        }
        Phase::Released => {
            if matches!(frame.motion_command, MotionCommand::VisualKick { .. }) {
                println!(
                    "ok: reached Ready pose, waited through Set and 10s restricted kickoff, resumed kicking"
                );
                markers.add(
                    clock.now,
                    Color32::LIGHT_GREEN,
                    "PASS: kicking after release",
                );
                exit.write(AppExit::Success);
                None
            } else if elapsed > Duration::from_secs(15) {
                Some("robot did not resume kicking within 15 seconds after release")
            } else {
                None
            }
        }
    };
    if let Some(reason) = failure {
        eprintln!(
            "opponent kickoff failed: {reason}; motion={:?}",
            frame.motion_command
        );
        markers.add(clock.now, Color32::RED, format!("FAIL: {reason}"));
        scenario.failed_at = Some(clock.now);
    }
}
