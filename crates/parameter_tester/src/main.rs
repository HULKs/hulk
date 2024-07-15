use std::{env::args, fs::File};

use color_eyre::{
    eyre::{Ok, WrapErr},
    Result,
};

use framework::Parameters as FrameworkParameters;
use parameters::directory::deserialize;
use serde_json::from_reader;
use types::hardware::Ids;

fn main() -> Result<()> {
    let framework_parameters_path = args()
        .nth(1)
        .unwrap_or("etc/parameters/framework.json".to_string());

    let file =
        File::open(framework_parameters_path).wrap_err("failed to open framework parameters")?;
    let framework_parameters: FrameworkParameters =
        from_reader(file).wrap_err("failed to parse framework parameters")?;

    let ids = Ids {
        body_id: String::new(),
        head_id: String::new(),
    };
    let _robotics_parameters: hulk::structs::Parameters =
        deserialize(framework_parameters.parameters_directory, &ids)?;

    Ok(())
}
