use std::collections::HashMap;

use color_eyre::{Result, eyre::Context};
use serde_json::{Value, to_value};

use crate::{Repository, modify_json::modify_json_inplace};

impl Repository {
    pub async fn configure_recording_intervals(
        &self,
        recording_intervals: HashMap<String, usize>,
    ) -> Result<()> {
        let framework_json = self.root.join("etc/parameters/framework.json");
        let serialized_intervals = to_value(recording_intervals)
            .wrap_err("failed to convert recording intervals to JSON")?;

        modify_json_inplace(&framework_json, |mut hardware_json: Value| {
            hardware_json["recording_intervals"] = serialized_intervals;
            hardware_json
        })
        .await
        .wrap_err_with(|| {
            format!(
                "failed to configure recording intervals in {}",
                framework_json.display()
            )
        })?;

        Ok(())
    }
}
