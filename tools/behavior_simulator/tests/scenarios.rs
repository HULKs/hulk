use std::{path::Path, time::Instant};

use color_eyre::{eyre::Context, Result};

use behavior_simulator::simulator::Simulator;

fn test_scenario(path: impl AsRef<Path>) -> Result<()> {
    let mut simulator = Simulator::try_new()?;
    simulator.execute_script(path)?;

    let start = Instant::now();
    simulator.run().wrap_err("failed to run simulation")?;
    let duration = Instant::now() - start;
    eprintln!("Took {:.2} seconds", duration.as_secs_f32());

    Ok(())
}
include!(concat!(env!("OUT_DIR"), "/behavior_files.rs"));

// #[test]
// fn test_ball_intercept() -> Result<()> {
//     test_scenario("../../tests/behavior/ball_intercept.lua")
// }

// #[test]
// fn test_ball_intercept_striker() -> Result<()> {
//     test_scenario("../../tests/behavior/ball_intercept_striker.lua")
// }

// #[test]
// fn test_defender_positioning() -> Result<()> {
//     test_scenario("../../tests/behavior/defender_positioning.lua")
// }

// #[test]
// fn test_demonstration() -> Result<()> {
//     test_scenario("../../tests/behavior/demonstration.lua")
// }

// #[test]
// fn test_golden_goal() -> Result<()> {
//     test_scenario("../../tests/behavior/golden_goal.lua")
// }

// #[test]
// fn test_golden_goal_opponent_kickoff() -> Result<()> {
//     test_scenario("../../tests/behavior/golden_goal_opponent_kickoff.lua")
// }

// #[test]
// fn test_golden_goal_striker_penalized() -> Result<()> {
//     test_scenario("../../tests/behavior/golden_goal_striker_penalized.lua")
// }

// #[test]
// fn test_hulks_vs_ghosts() -> Result<()> {
//     test_scenario("../../tests/behavior/hulks_vs_ghosts.lua")
// }

// #[test]
// fn test_ingamepenalty_attacking() -> Result<()> {
//     test_scenario("../../tests/behavior/ingamepenalty_attacking.lua")
// }

// #[test]
// fn test_ingamepenalty_defending() -> Result<()> {
//     test_scenario("../../tests/behavior/ingamepenalty_defending.lua")
// }

// #[test]
// fn test_ingamepenalty_definding_with_kick() -> Result<()> {
//     test_scenario("../../tests/behavior/ingamepenalty_defending_with_kick.lua")
// }

// #[test]
// fn test_oscillating_obstacle() -> Result<()> {
//     test_scenario("../../tests/behavior/oscillating_obstacle.lua")
// }

// #[test]
// fn test_quantum_ball() -> Result<()> {
//     test_scenario("../../tests/behavior/quantum_ball.lua")
// }

// #[test]
// fn test_reappearing_ball_in_front_of_1() -> Result<()> {
//     test_scenario("../../tests/behavior/reappearing_ball_in_front_of_1.lua")
// }

// #[test]
// fn test_replacementkeeper() -> Result<()> {
//     test_scenario("../../tests/behavior/replacementkeeper_test.lua")
// }

// #[test]
// fn test_vanishing_ball() -> Result<()> {
//     test_scenario("../../tests/behavior/vanishing_ball.lua")
// }

// #[test]
// fn test_walkaroundball() -> Result<()> {
//     test_scenario("../../tests/behavior/walkaroundball.lua")
// }
