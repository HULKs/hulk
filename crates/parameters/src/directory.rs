use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{error, from_str, from_value, to_string_pretty, to_value, Value};
use tokio::fs::{read_to_string, write};

use super::json::{copy_nested_value, merge_json, prune_equal_branches};

#[derive(Debug, thiserror::Error)]
pub enum DirectoryError {
    #[error("failed to get default parameters")]
    DefaultParametersNotGet(#[source] SerializationError),
    #[error("failed to get default parameters of location")]
    DefaultParametersOfLocationNotGet(#[source] SerializationError),
    #[error("failed to get body parameters")]
    BodyParametersNotGet(#[source] SerializationError),
    #[error("failed to get head parameters")]
    HeadParametersNotGet(#[source] SerializationError),
    #[error("failed to get body parameters of location")]
    BodyParametersOfLocationNotGet(#[source] SerializationError),
    #[error("failed to get head parameters of location")]
    HeadParametersOfLocationNotGet(#[source] SerializationError),
    #[error("failed to convert dynamic JSON object into resulting parameters object")]
    JsonValueNotConvertedToParameters(#[source] error::Error),
    #[error("failed to convert parameters object into dynamic JSON object")]
    ParametersNotConvertedToJsonValue(#[source] error::Error),
    #[error("failed to set head parameters of location")]
    HeadParametersOfLocationNotSet(#[source] SerializationError),
}

#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum SerializationError {
    #[error("failed to read {path:?}")]
    FileNotRead { source: io::Error, path: PathBuf },
    #[error("failed to parse {path:?}")]
    FileNotParsed { source: error::Error, path: PathBuf },
    #[error("failed to serialize {path:?}")]
    FileNotSerialized { source: error::Error, path: PathBuf },
    #[error("failed to write {path:?}")]
    FileNotWritten { source: io::Error, path: PathBuf },
}

pub async fn deserialize<Parameters>(
    parameters_root_path: impl AsRef<Path>,
    body_id: &str,
    head_id: &str,
) -> Result<Parameters, DirectoryError>
where
    Parameters: DeserializeOwned,
{
    let default_file_path = parameters_root_path.as_ref().join("default.json");
    let mut parameters = from_path(default_file_path)
        .await
        .map_err(DirectoryError::DefaultParametersNotGet)?;

    let location_directory = parameters_root_path
        .as_ref()
        .join(location_directory_from_head_id(head_id));

    let location_default_file_path = location_directory.join("default.json");
    if location_default_file_path.exists() {
        let location_default_parameters = from_path(location_default_file_path)
            .await
            .map_err(DirectoryError::DefaultParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_default_parameters);
    }

    let body_file_path = parameters_root_path
        .as_ref()
        .join(format!("body.{}.json", body_id));
    if body_file_path.exists() {
        let body_parameters = from_path(body_file_path)
            .await
            .map_err(DirectoryError::BodyParametersNotGet)?;
        merge_json(&mut parameters, &body_parameters);
    }

    let head_file_path = parameters_root_path
        .as_ref()
        .join(format!("head.{}.json", head_id));
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
    scope: Scope,
    path: &str,
    parameters_root_path: impl AsRef<Path>,
    body_id: &str,
    head_id: &str,
) -> Result<(), DirectoryError>
where
    Parameters: DeserializeOwned + Serialize,
{
    let mut parameters =
        to_value(parameters).map_err(DirectoryError::ParametersNotConvertedToJsonValue)?;
    let stored_parameters = to_value(
        deserialize::<Parameters>(&parameters_root_path, body_id, head_id)
            .await
            .map_err(|error| {
                println!("{:?}", error);
                error
            })?,
    )
    .map_err(DirectoryError::ParametersNotConvertedToJsonValue)?;

    prune_equal_branches(&mut parameters, &stored_parameters);

    let Some(sparse_parameters_from_scope_path) = copy_nested_value(&parameters, path) else {
        return Ok(());
    };
    let serialization_file_path =
        file_path_from_scope(scope, parameters_root_path, body_id, head_id);
    let mut parameters = if serialization_file_path.exists() {
        from_path(&serialization_file_path)
            .await
            .map_err(DirectoryError::HeadParametersOfLocationNotGet)?
    } else {
        Value::Object(Default::default())
    };
    merge_json(&mut parameters, &sparse_parameters_from_scope_path);

    to_path(serialization_file_path, parameters)
        .await
        .map_err(DirectoryError::HeadParametersOfLocationNotSet)
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scope {
    pub location: Location,
    pub id: Id,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Location {
    All,
    Current,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Id {
    All,
    Body,
    Head,
}

fn file_path_from_scope(
    scope: Scope,
    parameters_root_path: impl AsRef<Path>,
    body_id: &str,
    head_id: &str,
) -> PathBuf {
    let location_directory = parameters_root_path
        .as_ref()
        .join(location_directory_from_head_id(head_id));
    match (scope.location, scope.id) {
        (Location::All, Id::All) => parameters_root_path.as_ref().join("default.json"),
        (Location::All, Id::Body) => parameters_root_path
            .as_ref()
            .join(format!("body.{}.json", body_id)),
        (Location::All, Id::Head) => parameters_root_path
            .as_ref()
            .join(format!("head.{}.json", head_id)),
        (Location::Current, Id::All) => location_directory.join("default.json"),
        (Location::Current, Id::Body) => location_directory.join(format!("body.{}.json", body_id)),
        (Location::Current, Id::Head) => location_directory.join(format!("head.{}.json", head_id)),
    }
}

fn location_directory_from_head_id(head_id: &str) -> &'static str {
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
    let file_contents =
        to_string_pretty(&value).map_err(|source| SerializationError::FileNotSerialized {
            source,
            path: file_path.as_ref().to_path_buf(),
        })? + "\n";
    write(&file_path, file_contents.as_bytes())
        .await
        .map_err(|source| SerializationError::FileNotWritten {
            source,
            path: file_path.as_ref().to_path_buf(),
        })
}
