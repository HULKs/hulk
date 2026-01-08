use std::collections::HashSet;

use clap::Args;
use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};

use argument_parsers::RobotNumberPlayerAssignment;
use repository::Repository;

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// The assignments to change e.g. 20:2 or 32:5 (player numbers start from 1)
    #[arg(required = true)]
    pub assignments: Vec<RobotNumberPlayerAssignment>,
}

pub async fn player_number(arguments: Arguments, repository: &Repository) -> Result<()> {
    let team = repository
        .read_team_configuration()
        .await
        .wrap_err("failed to get team configuration")?;

    check_for_duplication(&arguments.assignments)?;

    // reborrows the team to avoid moving it into the closure
    let robots = &team.robots;

    ProgressIndicator::map_tasks(
        arguments.assignments,
        "Setting player number...",
        |assignment, _progress_bar| async move {
            let number = assignment.robot_number.number;
            let robot = robots
                .iter()
                .find(|robot| robot.number == number)
                .ok_or_else(|| eyre!("Robot with Hardware ID {number} does not exist"))?;
            repository
                .configure_player_number(&robot.head_id, assignment.player_number)
                .await
                .wrap_err_with(|| format!("failed to set player number for {assignment}"))
        },
    )
    .await;

    Ok(())
}

/// Check if two RobotNumbers are assigned to the same PlayerNumber
/// or if a RobotNumber is assigned to multiple PlayerNumbers
pub fn check_for_duplication(assignments: &[RobotNumberPlayerAssignment]) -> Result<()> {
    let mut existing_player_numbers = HashSet::new();
    let mut existing_robot_numbers = HashSet::new();

    if assignments.iter().any(
        |RobotNumberPlayerAssignment {
             robot_number,
             player_number,
         }| {
            !existing_robot_numbers.insert(robot_number)
                || !existing_player_numbers.insert(player_number)
        },
    ) {
        bail!("duplication in Robot to player number assignments")
    }
    Ok(())
}
