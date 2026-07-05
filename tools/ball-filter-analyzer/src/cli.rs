use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(about = "Analyze ball-filter traces from a robot MCAP recording")]
pub struct Args {
    pub recording: PathBuf,

    #[arg(long)]
    pub out: PathBuf,

    #[arg(long, default_value_t = 100)]
    pub match_window_ms: u64,

    #[arg(long, default_value_t = 0.3)]
    pub duplicate_distance_m: f32,

    #[arg(long, default_value_t = 16)]
    pub hypothesis_cap: usize,

    #[arg(long, default_value_t = 8)]
    pub assignment_pressure_threshold: usize,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::Args;

    #[test]
    fn parses_required_recording_and_output_directory() {
        let args = Args::parse_from([
            "ball-filter-analyzer",
            "/tmp/recording.mcap",
            "--out",
            "/tmp/analysis",
        ]);

        assert_eq!(args.recording, PathBuf::from("/tmp/recording.mcap"));
        assert_eq!(args.out, PathBuf::from("/tmp/analysis"));
        assert_eq!(args.match_window_ms, 100);
        assert_eq!(args.duplicate_distance_m, 0.3);
        assert_eq!(args.hypothesis_cap, 16);
        assert_eq!(args.assignment_pressure_threshold, 8);
    }
}
