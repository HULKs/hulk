use std::str::FromStr;

use clap::Args;
use color_eyre::{
    Result,
    eyre::{WrapErr, bail},
};

use robot::Robot;

use crate::progress_indicator::ProgressIndicator;
use argument_parsers::RobotAddress;

#[derive(Args, Debug)]
pub struct Arguments {
    #[arg(num_args = 1..)]
    pub arguments: Vec<String>,
}

pub async fn shell(arguments: Arguments) -> Result<()> {
    let mut iter = arguments.arguments.into_iter();
    let mut robots = Vec::new();
    let mut user_command = Vec::new();
    for item in iter.by_ref() {
        let Ok(address) = RobotAddress::from_str(&item) else {
            user_command.push(item);
            break;
        };
        robots.push(address);
    }
    user_command.extend(iter);

    if robots.len() == 1 {
        let mut command = Robot::try_new_with_ping(robots[0].ip)
            .await?
            .ssh_to_robot()?;
        command.arg("-t");
        let status = command
            .args(user_command)
            .status()
            .await
            .wrap_err("failed to execute shell ssh command")?;
        if !status.success() {
            bail!("shell ssh command exited with {status}");
        }
        return Ok(());
    }

    let progress = ProgressIndicator::new();

    progress
        .map_tasks(robots, "".to_string(), |robot, _progress_bar| {
            let user_command = user_command.clone();
            async move {
                let mut command = Robot::try_new_with_ping(robot.ip).await?.ssh_to_robot()?;
                command.args(&user_command);
                let output = command.output().await?;

                let stdout = String::from_utf8(output.stdout).wrap_err("stdout was not UTF-8")?;
                let stderr = String::from_utf8(output.stderr).wrap_err("stderr was not UTF-8")?;

                Ok(format!("{}\n{}{}", user_command.join(" "), stdout, stderr))
            }
        })
        .await;

    Ok(())
}
