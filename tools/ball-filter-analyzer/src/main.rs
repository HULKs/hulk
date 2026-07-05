use clap::Parser;
use color_eyre::Result;

use ball_filter_analyzer::{cli::Args, mcap_input::read_recording, parquet_output::write_outputs};

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let input = read_recording(&args.recording)?;
    write_outputs(&args.out, &input, &[])?;
    println!("wrote analysis to {}", args.out.display());
    Ok(())
}
