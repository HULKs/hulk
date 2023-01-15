use std::{
    io,
    path::{Path, PathBuf},
};

use log::debug;
use serde::de::DeserializeOwned;
use serde_json::{error, from_str, from_value, Value};
use tokio::fs::read_to_string;

#[derive(Debug, thiserror::Error)]
pub enum DirectoryError {
    #[error("failed to get default parameters")]
    DefaultParametersNotGet(DeserializeError),
    #[error("failed to get default parameters of location")]
    DefaultParametersOfLocationNotGet(DeserializeError),
    #[error("failed to get body parameters")]
    BodyParametersNotGet(DeserializeError),
    #[error("failed to get head parameters")]
    HeadParametersNotGet(DeserializeError),
    #[error("failed to get body parameters of location")]
    BodyParametersOfLocationNotGet(DeserializeError),
    #[error("failed to get head parameters of location")]
    HeadParametersOfLocationNotGet(DeserializeError),
    #[error("failed to convert dynamic JSON object into resulting parameters object")]
    JsonValueNotConvertedToParameters(error::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DeserializeError {
    #[error("failed to read {path:?}")]
    FileNotRead { source: io::Error, path: PathBuf },
    #[error("failed to parse {path:?}")]
    FileNotParsed { source: error::Error, path: PathBuf },
}

pub async fn deserialize<Parameters>(
    root_path: impl AsRef<Path>,
    body_id: &str,
    head_id: &str,
) -> Result<Parameters, DirectoryError>
where
    Parameters: DeserializeOwned,
{
    let default_file_path = root_path.as_ref().join("default.json");
    let mut parameters = from_path(default_file_path)
        .await
        .map_err(DirectoryError::DefaultParametersNotGet)?;

    let webots_id_found = head_id.starts_with("webots");
    let behavior_simulator_id_found = head_id.starts_with("behavior_simulator");
    let location_directory = if webots_id_found {
        "webots_location"
    } else if behavior_simulator_id_found {
        "behavior_simulator_location"
    } else {
        "nao_location"
    };

    let location_default_file_path = root_path
        .as_ref()
        .join(location_directory)
        .join("default.json");
    if location_default_file_path.exists() {
        let location_default_parameters = from_path(location_default_file_path)
            .await
            .map_err(DirectoryError::DefaultParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_default_parameters);
    }

    let body_file_path = root_path.as_ref().join(format!("body.{}.json", body_id));
    if body_file_path.exists() {
        let body_parameters = from_path(body_file_path)
            .await
            .map_err(DirectoryError::BodyParametersNotGet)?;
        merge_json(&mut parameters, &body_parameters);
    }

    let head_file_path = root_path.as_ref().join(format!("head.{}.json", head_id));
    if head_file_path.exists() {
        let head_parameters = from_path(head_file_path)
            .await
            .map_err(DirectoryError::HeadParametersNotGet)?;
        merge_json(&mut parameters, &head_parameters);
    }

    let location_body_file_path = root_path
        .as_ref()
        .join(location_directory)
        .join(format!("body.{}.json", body_id));
    if location_body_file_path.exists() {
        let location_body_parameters = from_path(location_body_file_path)
            .await
            .map_err(DirectoryError::BodyParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_body_parameters);
    }

    let location_head_file_path = root_path
        .as_ref()
        .join(location_directory)
        .join(format!("head.{}.json", head_id));
    if location_head_file_path.exists() {
        let location_head_parameters = from_path(location_head_file_path)
            .await
            .map_err(DirectoryError::BodyParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_head_parameters);
    }

    from_value(parameters).map_err(DirectoryError::JsonValueNotConvertedToParameters)
}

async fn from_path(file_path: impl AsRef<Path>) -> Result<Value, DeserializeError> {
    debug!("Reading {:?}...", file_path.as_ref());
    let file_contents =
        read_to_string(&file_path)
            .await
            .map_err(|source| DeserializeError::FileNotRead {
                source,
                path: file_path.as_ref().to_path_buf(),
            })?;
    from_str(&file_contents).map_err(|source| DeserializeError::FileNotParsed {
        source,
        path: file_path.as_ref().to_path_buf(),
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
