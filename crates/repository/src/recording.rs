use std::{collections::HashMap, path::Path};

use color_eyre::{eyre::Context, Result};
use serde_json::{to_value, Value};

use crate::modify_json::modify_json_inplace;

pub async fn set_recording_intervals(
    recording_intervals: HashMap<String, usize>,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    let framework_json = repository_root
        .as_ref()
        .join("etc/parameters/framework.json");
    let serialized_intervals =
        to_value(recording_intervals).wrap_err("failed to convert recording intervals to JSON")?;

    modify_json_inplace(framework_json, |mut hardware_json: Value| {
        hardware_json["recording_intervals"] = serialized_intervals;
        hardware_json
    })
    .await?;

    Ok(())
}
