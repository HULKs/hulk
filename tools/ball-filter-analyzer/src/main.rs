use clap::Parser;
use color_eyre::Result;

use ball_filter_analyzer::{cli::Args, mcap_input::read_recording};

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    std::fs::create_dir_all(&args.out)?;
    let input = read_recording(&args.recording)?;
    println!("decoded {} track rows", input.tracks.len());
    println!("decoded {} percept rows", input.percepts.len());
    println!("decode errors: {}", input.decode_errors.len());
    Ok(())
}
