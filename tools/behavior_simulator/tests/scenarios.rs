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

#[test]
fn test_golden_goal() -> Result<()> {
    test_scenario("../../tests/behavior/golden_goal.lua")
}

#[test]
fn test_demonstration() -> Result<()> {
    test_scenario("../../tests/behavior/demonstration.lua")
}
