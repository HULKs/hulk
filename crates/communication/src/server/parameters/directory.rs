use std::{
    io,
    path::{Path, PathBuf},
};

use log::debug;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{error, from_str, from_value, to_value, Value};
use tokio::fs::{read_to_string, write};

#[derive(Debug, thiserror::Error)]
pub enum DirectoryError {
    #[error("failed to get default parameters")]
    DefaultParametersNotGet(SerializationError),
    #[error("failed to get default parameters of location")]
    DefaultParametersOfLocationNotGet(SerializationError),
    #[error("failed to get body parameters")]
    BodyParametersNotGet(SerializationError),
    #[error("failed to get head parameters")]
    HeadParametersNotGet(SerializationError),
    #[error("failed to get body parameters of location")]
    BodyParametersOfLocationNotGet(SerializationError),
    #[error("failed to get head parameters of location")]
    HeadParametersOfLocationNotGet(SerializationError),
    #[error("failed to convert dynamic JSON object into resulting parameters object")]
    JsonValueNotConvertedToParameters(error::Error),
    #[error("failed to convert parameters object into dynamic JSON object")]
    ParametersNotConvertedToJsonValue(error::Error),
    #[error("failed to set head parameters of location")]
    HeadParametersOfLocationNotSet(SerializationError),
}

#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum SerializationError {
    #[error("failed to read {path:?}")]
    FileNotRead { source: io::Error, path: PathBuf },
    #[error("failed to parse {path:?}")]
    FileNotParsed { source: error::Error, path: PathBuf },
    #[error("failed to write {path:?}")]
    FileNotWritten { source: io::Error, path: PathBuf },
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

    let location_directory = root_path.as_ref().join(get_location_directory(head_id));

    let location_default_file_path = location_directory.join("default.json");
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

    let location_body_file_path = location_directory.join(format!("body.{}.json", body_id));
    if location_body_file_path.exists() {
        let location_body_parameters = from_path(location_body_file_path)
            .await
            .map_err(DirectoryError::BodyParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_body_parameters);
    }

    let location_head_file_path = location_directory.join(format!("head.{}.json", head_id));
    if location_head_file_path.exists() {
        let location_head_parameters = from_path(location_head_file_path)
            .await
            .map_err(DirectoryError::HeadParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_head_parameters);
    }

    from_value(parameters).map_err(DirectoryError::JsonValueNotConvertedToParameters)
}

pub async fn serialize<Parameters>(
    parameters: &Parameters,
    root_path: impl AsRef<Path>,
    body_id: &str,
    head_id: &str,
) -> Result<(), DirectoryError>
where
    Parameters: Serialize,
{
    let mut parameters =
        to_value(parameters).map_err(DirectoryError::ParametersNotConvertedToJsonValue)?;
    let stored_parameters = to_value(deserialize(&root_path, body_id, head_id).await?)
        .map_err(DirectoryError::ParametersNotConvertedToJsonValue)?;

    prune_equal_branches(&mut parameters, &stored_parameters);

    let location_head_file_path = root_path
        .as_ref()
        .join(get_location_directory(head_id))
        .join(format!("head.{}.json", head_id));

    let mut location_head_parameters = from_path(&location_head_file_path)
        .await
        .map_err(DirectoryError::BodyParametersOfLocationNotGet)?;
    merge_json(&mut location_head_parameters, &parameters);

    to_path(location_head_file_path, location_head_parameters)
        .await
        .map_err(DirectoryError::HeadParametersOfLocationNotSet)
}

fn get_location_directory(head_id: &str) -> &'static str {
    let webots_id_found = head_id.starts_with("webots");
    let behavior_simulator_id_found = head_id.starts_with("behavior_simulator");
    if webots_id_found {
        "webots_location"
    } else if behavior_simulator_id_found {
        "behavior_simulator_location"
    } else {
        "nao_location"
    }
}

async fn from_path(file_path: impl AsRef<Path>) -> Result<Value, SerializationError> {
    debug!("Reading {:?}...", file_path.as_ref());
    let file_contents =
        read_to_string(&file_path)
            .await
            .map_err(|source| SerializationError::FileNotRead {
                source,
                path: file_path.as_ref().to_path_buf(),
            })?;
    from_str(&file_contents).map_err(|source| SerializationError::FileNotParsed {
        source,
        path: file_path.as_ref().to_path_buf(),
    })
}

async fn to_path(file_path: impl AsRef<Path>, value: Value) -> Result<(), SerializationError> {
    debug!("Writing {:?}...", file_path.as_ref());
    let file_contents = value.to_string();
    write(&file_path, file_contents.as_bytes())
        .await
        .map_err(|source| SerializationError::FileNotWritten {
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

fn prune_equal_branches(own: &mut Value, other: &Value) {
    if own == other {
        *own = Value::Object(Default::default());
        return;
    }
    if let (&mut Value::Object(ref mut own), &Value::Object(ref other)) = (own, other) {
        let mut keys_to_remove = vec![];
        for (key, own_value) in own.iter_mut() {
            if let Some(other_value) = other.get(key) {
                if own_value == other_value {
                    keys_to_remove.push(key.clone());
                    continue;
                }
                prune_equal_branches(own_value, other_value);
            }
        }
        for key in keys_to_remove {
            own.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_value_is_set_to_an_object() {
        let mut own = Value::Null;
        let other = Value::Null;

        prune_equal_branches(&mut own, &other);

        assert_eq!(own, Value::Object(Default::default()));
    }

    #[test]
    fn different_types_are_kept() {
        let mut own: Value = from_str(r#"{"a":42,"b":true,"c":null}"#).unwrap();
        let original_own = own.clone();
        let other: Value = from_str(r#"{"a":true,"b":null,"c":42}"#).unwrap();

        prune_equal_branches(&mut own, &other);

        assert_eq!(own, original_own);
    }

    #[test]
    fn only_deep_leafs_are_kept() {
        let mut own: Value = from_str(r#"{"a":{"b":{"c":42},"d":{"e":1337}}}"#).unwrap();
        let other: Value = from_str(r#"{"a":{"b":{"c":true},"d":{"e":1337}}}"#).unwrap();

        prune_equal_branches(&mut own, &other);

        assert_eq!(own, from_str::<Value>(r#"{"a":{"b":{"c":42}}}"#).unwrap());
    }
}
