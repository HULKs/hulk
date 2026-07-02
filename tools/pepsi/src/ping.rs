use std::time::Duration;

use clap::Args;

use argument_parsers::RobotAddress;
use color_eyre::owo_colors::OwoColorize;
use robot::Robot;
use tokio::time::{Instant, sleep};

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// Timeout in seconds after which ping is aborted
    #[arg(long, short, default_value = "1")]
    pub timeout: f64,
    /// Repeat ping indefinitely
    #[arg(long, short)]
    pub watch: bool,
    /// Interval in seconds between ping attempts when watching
    #[arg(long, short, default_value = "1")]
    pub interval: f64,
    /// The Robots to ping to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub robots: Vec<RobotAddress>,
}

pub async fn ping(arguments: Arguments) {
    let timeout = Duration::from_secs_f64(arguments.timeout);
    let interval = Duration::from_secs_f64(arguments.interval);
    ProgressIndicator::new()
        .map_tasks(
            arguments.robots,
            "Pinging Robot...",
            |robot_address, progress_bar| async move {
                let mut last_change = Instant::now();
                let mut last_success = false;
                loop {
                    let ping_start = Instant::now();
                    let result = Robot::try_new_with_ping_and_arguments(robot_address.ip, timeout)
                        .await
                        .map(|_| ());
                    let ping_duration = ping_start.elapsed();

                    if !arguments.watch {
                        return result;
                    }

                    match &result {
                        Ok(_) => {
                            if !last_success {
                                last_change = Instant::now();
                            }
                            let message = format!(
                                "{} since {}s",
                                "✔".green(),
                                last_change.elapsed().as_secs()
                            );
                            progress_bar.set_message(message);
                        }
                        Err(report) => {
                            if last_success {
                                last_change = Instant::now();
                            }
                            let message = format!(
                                "{} {report} since {}s",
                                "✗".red(),
                                last_change.elapsed().as_secs()
                            );
                            progress_bar.set_message(message);
                        }
                    };
                    last_success = result.is_ok();
                    sleep(interval.saturating_sub(ping_duration)).await;
                }
            },
        )
        .await;
}
