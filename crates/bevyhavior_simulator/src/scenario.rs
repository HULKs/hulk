use clap::Parser;

#[derive(Parser)]
pub struct Arguments {
    /// Just run the simulation, don't serve the result
    #[arg(short, long)]
    pub run: bool,
}
