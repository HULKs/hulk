use std::time::Duration;

use bevyhavior_simulator::behavior_tree_simulator::Simulation;
use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, point, vector};
use types::{
    field_dimensions::FieldDimensions, motion_command::MotionCommand,
    parameters::BehaviorParameters, primary_state::PrimaryState,
};

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut parameters = BehaviorParameters {
        goal_keeper_number: PlayerNumber::One,
        ..Default::default()
    };
    parameters.last_ball_timeout = Duration::from_secs(2);

    let mut simulation = Simulation::new(FieldDimensions::SPL_2025);
    simulation.spawn_robot(PlayerNumber::Three, pose(0.0, 0.0, 0.0), parameters.clone())?;
    simulation.spawn_robot(PlayerNumber::Four, pose(-1.0, 1.0, 0.0), parameters)?;
    simulation.set_primary_state(PrimaryState::Playing);
    simulation.set_ball(point![1.0, 0.0], vector![0.0, 0.0]);

    simulation.run_for(Duration::from_secs(2))?;

    for (index, frame) in simulation.timeline.iter().enumerate() {
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

    println!(
        "result={} frames={}",
        if simulation.failed { "failed" } else { "ok" },
        simulation.timeline.len()
    );

    Ok(())
}

fn pose(x: f32, y: f32, yaw: f32) -> Isometry2<Ground, Field> {
    Isometry2::from_parts(vector![x, y], yaw)
}

fn motion_name(motion_command: &MotionCommand) -> &'static str {
    match motion_command {
        MotionCommand::Prepare => "prepare",
        MotionCommand::Stand { .. } => "stand",
        MotionCommand::StandUp => "stand_up",
        MotionCommand::VisualKick { .. } => "visual_kick",
        MotionCommand::Walk { .. } => "walk",
        MotionCommand::WalkWithVelocity { .. } => "walk_with_velocity",
    }
}
