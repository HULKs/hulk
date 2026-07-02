use std::time::Duration;

use clap::Args;

use argument_parsers::RobotAddress;
use color_eyre::owo_colors::OwoColorize;
use robot::Robot;
use tokio::time::Instant;

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// Timeout in seconds after which ping is aborted
    #[arg(long, short, default_value = "2")]
    pub timeout: u64,
    /// Repeat ping indefinitely
    #[arg(long, short)]
    pub watch: bool,
    /// The Robots to ping to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub robots: Vec<RobotAddress>,
}

pub async fn ping(arguments: Arguments) {
    ProgressIndicator::new()
        .map_tasks(
            arguments.robots,
            "Pinging Robot...",
            |robot_address, progress_bar| async move {
                let mut last_change = Instant::now();
                let mut last_success = false;
                loop {
                    let result = Robot::try_new_with_ping_and_arguments(
                        robot_address.ip,
                        Duration::from_secs(arguments.timeout),
                    )
                    .await
                    .map(|_| ());

                    if !arguments.watch {
                        return result;
                    }

                    let message = match &result {
                        Ok(_) => {
                            if !last_success {
                                last_change = Instant::now();
                            }
                            format!("{} since {}s", "✔".green(), last_change.elapsed().as_secs())
                        }
                        Err(report) => {
                            if last_success {
                                last_change = Instant::now();
                            }
                            format!(
                                "{} {report} since {}s",
                                "✗".red(),
                                last_change.elapsed().as_secs()
                            )
                        }
                    };
                    last_success = result.is_ok();
                    progress_bar.set_message(message);
                }
            },
        )
        .await;
}
