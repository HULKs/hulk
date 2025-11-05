use std::{
    env::{args, set_current_dir},
    fs::File,
    path::PathBuf,
};

use color_eyre::{
    eyre::{ContextCompat, Ok, WrapErr},
    Result,
};
use serde_json::from_reader;

use framework::Parameters as FrameworkParameters;
use hula_types::hardware::Ids;
use parameters::directory::deserialize;
use repository::Repository;

fn main() -> Result<()> {
    let repository_search_path = match args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from("."),
    };
    let repository =
        Repository::find_root(repository_search_path).wrap_err("no repository found")?;
    set_current_dir(repository.root).wrap_err("failed to cd to repo root")?;

    let file = File::open("etc/parameters/framework.json")
        .wrap_err("failed to open framework parameters")?;
    let framework_parameters: FrameworkParameters =
        from_reader(file).wrap_err("failed to parse framework parameters")?;

    let ids = Ids {
        body_id: String::new(),
        head_id: String::new(),
    };
    let _robotics_parameters: structs::Parameters =
        deserialize(framework_parameters.parameters_directory, &ids, false)?;

    Ok(())
}

#[allow(dead_code)]
mod structs {
    include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));
}
