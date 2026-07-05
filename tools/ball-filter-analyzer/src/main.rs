use clap::Parser;
use color_eyre::Result;

use ball_filter_analyzer::{
    analysis::{AnalysisConfig, detect_events},
    cli::Args,
    mcap_input::read_recording,
    parquet_output::write_outputs,
    report::write_report,
};

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let input = read_recording(&args.recording)?;
    let config = AnalysisConfig {
        match_window_ns: i64::try_from(args.match_window_ms.saturating_mul(1_000_000))
            .unwrap_or(i64::MAX),
        duplicate_distance_m: args.duplicate_distance_m,
        hypothesis_cap: i64::try_from(args.hypothesis_cap).unwrap_or(i64::MAX),
        assignment_pressure_threshold: i64::try_from(args.assignment_pressure_threshold)
            .unwrap_or(i64::MAX),
    };
    let events = detect_events(&input, &config);
    write_outputs(&args.out, &input, &events)?;
    write_report(&args.out.join("report.md"), &input, &events)?;
    println!("wrote analysis to {}", args.out.display());
    Ok(())
}
