use std::{fs, path::Path};

use serde_json::{Map, Value};

use super::{ParameterError, Result};

pub fn load_json5_object_or_empty(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }

    let raw = fs::read_to_string(path).map_err(|err| ParameterError::FileReadError {
        path: path.to_path_buf(),
        source: err,
    })?;

    let value = json5::from_str::<Value>(&raw).map_err(|err| ParameterError::ParseError {
        path: path.to_path_buf(),
        source: err,
    })?;

    if !value.is_object() {
        return Err(ParameterError::ValidationError {
            message: format!(
                "top-level parameter value in {} must be an object",
                path.display()
            ),
        });
    }

    Ok(value)
}
