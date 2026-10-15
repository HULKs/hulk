use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::Result;

mod recording;
#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(about = "Inspect indexed ROS-Z MCAP recording metadata")]
struct Arguments {
    file: PathBuf,
    #[arg(long, default_value = "/replay")]
    namespace: String,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Arguments::parse();
    let recording = recording::Recording::open(&args.file, &args.namespace)?;
    println!(
        "{} topics, log-time bounds {}..{}",
        recording.channels.len(),
        recording.start,
        recording.end
    );
    Ok(())
}
