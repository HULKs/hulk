use color_eyre::{Result, eyre::Context};
use serde_json::Value;

use crate::{Repository, modify_json::modify_json_inplace};

impl Repository {
    pub async fn configure_communication(&self, enable: bool) -> Result<()> {
        let framework_json = self.root.join("etc/parameters/framework.json");

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
}
