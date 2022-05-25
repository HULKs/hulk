use std::{fs::File, path::Path};

use anyhow::Context;
use log::debug;
use serde_json::{from_reader, Value};

use crate::hardware::HardwareIds;

pub fn deserialize<P: AsRef<Path>>(root_path: P, ids: HardwareIds) -> anyhow::Result<Value> {
    let default_file_path = root_path.as_ref().join("default.json");
    let mut configuration = from_path(default_file_path)?;

    let webots_id_found = ids.head_id.starts_with("webots");
    let simulated_behavior_id_found = ids.head_id.starts_with("simulated_behavior");
    let location_directory = if webots_id_found {
        "webots_location"
    } else if simulated_behavior_id_found {
        "simulated_behavior"
    } else {
        "nao_location"
    };

    let location_default_file_path = root_path
        .as_ref()
        .join(location_directory)
        .join("default.json");
    if location_default_file_path.exists() {
        let location_default_configuration = from_path(location_default_file_path)?;
        merge_json(&mut configuration, &location_default_configuration);
    }

    let body_file_path = root_path
        .as_ref()
        .join(format!("body.{}.json", ids.body_id));
    if body_file_path.exists() {
        let body_configuration = from_path(body_file_path)?;
        merge_json(&mut configuration, &body_configuration);
    }

    let head_file_path = root_path
        .as_ref()
        .join(format!("head.{}.json", ids.head_id));
    if head_file_path.exists() {
        let head_configuration = from_path(head_file_path)?;
        merge_json(&mut configuration, &head_configuration);
    }

    let location_body_file_path = root_path
        .as_ref()
        .join(location_directory)
        .join(format!("body.{}.json", ids.body_id));
    if location_body_file_path.exists() {
        let location_body_configuration = from_path(location_body_file_path)?;
        merge_json(&mut configuration, &location_body_configuration);
    }

    let location_head_file_path = root_path
        .as_ref()
        .join(location_directory)
        .join(format!("head.{}.json", ids.head_id));
    if location_head_file_path.exists() {
        let location_head_configuration = from_path(location_head_file_path)?;
        merge_json(&mut configuration, &location_head_configuration);
    }

    Ok(configuration)
}

fn from_path<P: AsRef<Path>>(file_path: P) -> anyhow::Result<Value> {
    debug!("Reading {}...", file_path.as_ref().display());
    let location_head_file = File::open(&file_path).with_context(|| {
        format!(
            "Failed to open configuration file {}",
            file_path.as_ref().display()
        )
    })?;
    from_reader(&location_head_file).with_context(|| {
        format!(
            "Failed to parse configuration file {}",
            file_path.as_ref().display()
        )
    })
}

fn merge_json(own: &mut Value, other: &Value) {
    match (own, other) {
        (&mut Value::Object(ref mut own), &Value::Object(ref other)) => {
            for (key, value) in other {
                merge_json(own.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (own, other) => {
            *own = other.clone();
        }
    }
}
