use std::{
    fs::{read_to_string, write},
    io,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{error, from_str, from_value, to_string_pretty, to_value, Value};
use types::hardware::Ids;

use super::json::{clone_nested_value, merge_json, prune_equal_branches};

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

pub fn deserialize<Parameters>(
    parameters_root_path: impl AsRef<Path>,
    hardware_ids: &Ids,
) -> Result<Parameters, DirectoryError>
where
    Parameters: DeserializeOwned,
{
    let default_file_path = parameters_root_path.as_ref().join("default.json");
    let mut parameters =
        read_from_file(default_file_path).map_err(DirectoryError::DefaultParametersNotGet)?;

    let location_directory = parameters_root_path
        .as_ref()
        .join(location_directory_from_head_id(&hardware_ids.head_id));

    let location_default_file_path = location_directory.join("default.json");
    if location_default_file_path.exists() {
        let location_default_parameters = read_from_file(location_default_file_path)
            .map_err(DirectoryError::DefaultParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_default_parameters);
    }

    let body_file_path = parameters_root_path
        .as_ref()
        .join(format!("body.{}.json", &hardware_ids.body_id));
    if body_file_path.exists() {
        let body_parameters =
            read_from_file(body_file_path).map_err(DirectoryError::BodyParametersNotGet)?;
        merge_json(&mut parameters, &body_parameters);
    }

    let head_file_path = parameters_root_path
        .as_ref()
        .join(format!("head.{}.json", &hardware_ids.head_id));
    if head_file_path.exists() {
        let head_parameters =
            read_from_file(head_file_path).map_err(DirectoryError::HeadParametersNotGet)?;
        merge_json(&mut parameters, &head_parameters);
    }

    let location_body_file_path =
        location_directory.join(format!("body.{}.json", &hardware_ids.body_id));
    if location_body_file_path.exists() {
        let location_body_parameters = read_from_file(location_body_file_path)
            .map_err(DirectoryError::BodyParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_body_parameters);
    }

    let location_head_file_path =
        location_directory.join(format!("head.{}.json", &hardware_ids.head_id));
    if location_head_file_path.exists() {
        let location_head_parameters = read_from_file(location_head_file_path)
            .map_err(DirectoryError::HeadParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_head_parameters);
    }

    from_value(parameters).map_err(DirectoryError::JsonValueNotConvertedToParameters)
}

pub fn serialize<Parameters>(
    parameters: &Parameters,
    scope: Scope,
    path: &str,
    parameters_root_path: impl AsRef<Path>,
    hardware_ids: &Ids,
) -> Result<(), DirectoryError>
where
    for<'de> Parameters: Deserialize<'de> + Serialize,
{
    let mut parameters =
        to_value(parameters).map_err(DirectoryError::ParametersNotConvertedToJsonValue)?;
    let stored_parameters = to_value(
        deserialize::<Parameters>(&parameters_root_path, hardware_ids).map_err(|error| {
            println!("{:?}", error);
            error
        })?,
    )
    .map_err(DirectoryError::ParametersNotConvertedToJsonValue)?;

    prune_equal_branches(&mut parameters, &stored_parameters);

    let Some(sparse_parameters_from_scope_path) = clone_nested_value(&parameters, path) else {
        return Ok(());
    };
    let serialization_file_path = file_path_from_scope(scope, parameters_root_path, hardware_ids);
    let mut parameters = if serialization_file_path.exists() {
        read_from_file(&serialization_file_path)
            .map_err(DirectoryError::HeadParametersOfLocationNotGet)?
    } else {
        Value::Object(Default::default())
    };
    merge_json(&mut parameters, &sparse_parameters_from_scope_path);

    write_to_file(serialization_file_path, parameters)
        .map_err(DirectoryError::HeadParametersOfLocationNotSet)
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scope {
    pub location: Location,
    pub id: Id,
}

impl Scope {
    pub fn current_head() -> Self {
        Self {
            location: Location::Current,
            id: Id::Head,
        }
    }

    pub fn current_body() -> Self {
        Self {
            location: Location::Current,
            id: Id::Body,
        }
    }
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
    hardware_ids: &Ids,
) -> PathBuf {
    let directory = match scope.location {
        Location::All => parameters_root_path.as_ref().to_path_buf(),
        Location::Current => parameters_root_path
            .as_ref()
            .join(location_directory_from_head_id(&hardware_ids.head_id)),
    };
    match scope.id {
        Id::All => directory.join("default.json"),
        Id::Body => directory.join(format!("body.{}.json", &hardware_ids.body_id)),
        Id::Head => directory.join(format!("head.{}.json", &hardware_ids.head_id)),
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

fn read_from_file(file_path: impl AsRef<Path>) -> Result<Value, SerializationError> {
    let file_contents =
        read_to_string(&file_path).map_err(|source| SerializationError::FileNotRead {
            source,
            path: file_path.as_ref().to_path_buf(),
        })?;
    from_str(&file_contents).map_err(|source| SerializationError::FileNotParsed {
        source,
        path: file_path.as_ref().to_path_buf(),
    })
}

fn write_to_file(file_path: impl AsRef<Path>, value: Value) -> Result<(), SerializationError> {
    let file_contents =
        to_string_pretty(&value).map_err(|source| SerializationError::FileNotSerialized {
            source,
            path: file_path.as_ref().to_path_buf(),
        })? + "\n";
    write(&file_path, file_contents.as_bytes()).map_err(|source| {
        SerializationError::FileNotWritten {
            source,
            path: file_path.as_ref().to_path_buf(),
        }
    })
}
