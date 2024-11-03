use std::path::Path;

use color_eyre::Result;
use serde_json::Value;

use crate::modify_json::modify_json_inplace;

/// Sets the communication address in the framework.json file.
///
/// This function takes a boolean value to enable or disable communication. It reads the
/// framework.json file, updates the communication address field, and writes the updated JSON back.
pub async fn set_communication(enable: bool, repository_root: impl AsRef<Path>) -> Result<()> {
    let framework_json = repository_root
        .as_ref()
        .join("etc/parameters/framework.json");

    let address = if enable {
        Value::String("[::]:1337".to_string())
    } else {
        Value::Null
    };

    modify_json_inplace(framework_json, |mut hardware_json: Value| {
        hardware_json["communication_addresses"] = address;
        hardware_json
    })
    .await?;

    Ok(())
}
