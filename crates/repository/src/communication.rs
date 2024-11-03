use std::path::Path;

use color_eyre::{eyre::Context, Result};
use serde_json::Value;

use crate::modify_json::modify_json_inplace;

pub async fn configure_communication(
    enable: bool,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    let framework_json = repository_root
        .as_ref()
        .join("etc/parameters/framework.json");

    let address = if enable {
        Value::String("[::]:1337".to_string())
    } else {
        Value::Null
    };

    modify_json_inplace(&framework_json, |mut hardware_json: Value| {
        hardware_json["communication_addresses"] = address;
        hardware_json
    })
    .await
    .wrap_err_with(|| {
        format!(
            "failed to configure communication address in {}",
            framework_json.display()
        )
    })?;

    Ok(())
}
