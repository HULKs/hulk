use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::Result;
use bincode::deserialize_from;
use clap::Parser;
use control::localization_recorder::RecordedCycleContext;

#[derive(Parser)]
struct Arguments {
    log_file: PathBuf,
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();
    println!("{:?}", arguments.log_file);
    let mut reader = BufReader::new(File::open(arguments.log_file)?);
    for _ in 0..5 {
        let data: RecordedCycleContext = deserialize_from(&mut reader)?;
        println!("{data:?}");
    }

    Ok(())
}
