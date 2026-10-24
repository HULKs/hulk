use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use ros_z::prelude::*;
use tokio::{
    process::Command,
    time::{Interval, MissedTickBehavior, interval},
};
use types::system_info::{DiskIo, NetworkIo, Tegrastats};

mod io;
mod tegrastats;

pub const SAMPLE_INTERVAL: Duration = Duration::from_millis(200);

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let mut command = Command::new("/usr/bin/tegrastats");
    command
        .arg("--interval")
        .arg(SAMPLE_INTERVAL.as_millis().to_string());
    run_with_command(ctx, command).await
}

async fn run_with_command(ctx: Arc<Context>, command: Command) -> Result<()> {
    let node = ctx.create_node("system_info").build().await?;
    let tegrastats = node
        .publisher::<Tegrastats>("system/tegrastats")
        .build()
        .await?;
    let disk = node
        .publisher::<BTreeMap<String, DiskIo>>("system/disk_io")
        .build()
        .await?;
    let network = node
        .publisher::<BTreeMap<String, NetworkIo>>("system/network_io")
        .build()
        .await?;
    tokio::try_join!(
        tegrastats::run(tegrastats, command),
        io::run_disk(disk),
        io::run_network(network),
    )?;
    Ok(())
}

fn sample_timer() -> Interval {
    let mut timer = interval(SAMPLE_INTERVAL);
    timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
    timer
}
