use std::{
    collections::BTreeSet,
    fs::{read_to_string, write},
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, error, from_str, to_string_pretty, to_value};

use hula_types::hardware::Ids;

use super::json::{clone_nested_value, merge_json, prune_equal_branches};

#[derive(Debug, thiserror::Error)]
pub enum DirectoryError {
    #[error("failed to get default parameters")]
    DefaultParametersNotGet(#[source] SerializationError),
    #[error("failed to get default parameters of location")]
    DefaultParametersOfLocationNotGet(#[source] SerializationError),
    #[error("failed to get robot parameters")]
    RobotParametersNotGet(#[source] SerializationError),
    #[error("failed to get robot parameters of location")]
    RobotParametersOfLocationNotGet(#[source] SerializationError),
    #[error("failed to convert dynamic JSON object into resulting parameters object")]
    JsonValueNotConvertedToParameters(#[source] error::Error),
    #[error("failed to convert parameters object into dynamic JSON object")]
    ParametersNotConvertedToJsonValue(#[source] error::Error),
    #[error("failed to set robot parameters of location")]
    RobotParametersOfLocationNotSet(#[source] SerializationError),
    #[error("superfluous fields in json: {field_names:#?}")]
    SuperfluousFields { field_names: BTreeSet<String> },
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
    allow_superfluous_fields: bool,
) -> Result<Parameters, DirectoryError>
where
    Parameters: DeserializeOwned,
{
    let default_file_path = parameters_root_path.as_ref().join("default.json");
    let mut parameters =
        read_from_file(default_file_path).map_err(DirectoryError::DefaultParametersNotGet)?;

    let location_directory = parameters_root_path
        .as_ref()
        .join(location_directory_from_id(&hardware_ids.robot_id));

    let location_default_file_path = location_directory.join("default.json");
    if location_default_file_path.exists() {
        let location_default_parameters = read_from_file(location_default_file_path)
            .map_err(DirectoryError::DefaultParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_default_parameters);
    }

    let robot_file_path = parameters_root_path
        .as_ref()
        .join(format!("robot.{}.json", &hardware_ids.robot_id));
    if robot_file_path.exists() {
        let robot_parameters =
            read_from_file(robot_file_path).map_err(DirectoryError::RobotParametersNotGet)?;
        merge_json(&mut parameters, &robot_parameters);
    }

    let location_robot_file_path =
        location_directory.join(format!("robot.{}.json", &hardware_ids.robot_id));
    if location_robot_file_path.exists() {
        let location_robot_parameters = read_from_file(location_robot_file_path)
            .map_err(DirectoryError::RobotParametersOfLocationNotGet)?;
        merge_json(&mut parameters, &location_robot_parameters);
    }

    let mut superfluous_fields = BTreeSet::<String>::new();
    let parsed = serde_ignored::deserialize(parameters, |path| {
        superfluous_fields.insert(path.to_string());
    })
    .map_err(DirectoryError::JsonValueNotConvertedToParameters);
    if !allow_superfluous_fields && !superfluous_fields.is_empty() {
        return Err(DirectoryError::SuperfluousFields {
            field_names: superfluous_fields,
        });
    }
    parsed
}

pub fn serialize<Parameters>(
    parameters: &Parameters,
    scope: Scope,
    path: &str,
    parameters_root: impl AsRef<Path>,
    hardware_ids: &Ids,
) -> Result<(), DirectoryError>
where
    for<'de> Parameters: Deserialize<'de> + Serialize,
{
    let mut parameters =
        to_value(parameters).map_err(DirectoryError::ParametersNotConvertedToJsonValue)?;
    let stored_parameters = to_value(
        deserialize::<Parameters>(&parameters_root, hardware_ids, true).map_err(|error| {
            println!("{error:?}");
            error
        })?,
    )
    .map_err(DirectoryError::ParametersNotConvertedToJsonValue)?;

    prune_equal_branches(&mut parameters, &stored_parameters);

    let Some(sparse_parameters_from_scope_path) = clone_nested_value(&parameters, path) else {
        return Ok(());
    };
    let serialization_file_path = file_path_from_scope(scope, parameters_root, hardware_ids);
    let mut parameters = if serialization_file_path.exists() {
        read_from_file(&serialization_file_path)
            .map_err(DirectoryError::RobotParametersOfLocationNotGet)?
    } else {
        Value::Object(Default::default())
    };
    merge_json(&mut parameters, &sparse_parameters_from_scope_path);

    write_to_file(serialization_file_path, parameters)
        .map_err(DirectoryError::RobotParametersOfLocationNotSet)
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scope {
    pub location: Location,
    pub id: Id,
}

impl Scope {
    pub fn default_location() -> Self {
        Self {
            location: Location::All,
            id: Id::All,
        }
    }

    pub fn default_robot() -> Self {
        Self {
            location: Location::All,
            id: Id::Robot,
        }
    }

    pub fn current_location() -> Self {
        Self {
            location: Location::Current,
            id: Id::All,
        }
    }

    pub fn current_robot() -> Self {
        Self {
            location: Location::Current,
            id: Id::Robot,
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
    Robot,
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
            .join(location_directory_from_id(&hardware_ids.robot_id)),
    };
    match scope.id {
        Id::All => directory.join("default.json"),
        Id::Robot => directory.join(format!("robot.{}.json", &hardware_ids.robot_id)),
    }
}

fn location_directory_from_id(id: &str) -> &'static str {
    let mujoco_id_found = id.starts_with("mujoco");
    let behavior_simulator_id_found = id.starts_with("behavior_simulator");
    if mujoco_id_found {
        "mujoco_location"
    } else if behavior_simulator_id_found {
        "behavior_simulator_location"
    } else {
        "booster_location"
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
