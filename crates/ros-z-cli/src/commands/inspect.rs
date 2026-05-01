use color_eyre::eyre::{Result, eyre};

use crate::{
    cli::InspectArgs,
    render::{OutputMode, json, text},
};

pub fn run(output_mode: OutputMode, args: &InspectArgs) -> Result<()> {
    let report = ros_z_record::inspect_file(&args.input).map_err(|error| eyre!(error))?;

    match output_mode {
        OutputMode::Json => json::print_pretty(&report),
        OutputMode::Text => {
            text::print_inspection_report(&report);
            Ok(())
        }
    }
}
