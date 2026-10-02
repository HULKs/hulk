use clap::Parser;
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ball_filter_tuner::run(ball_filter_tuner::Args::parse())
}
