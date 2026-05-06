use std::{fs, path::Path};

use serde_json::{Map, Value};

use super::{ParameterError, Result};

pub fn load_json5_object_or_empty(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }

    let raw = fs::read_to_string(path).map_err(|err| ParameterError::FileReadError {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;

    let value = json5::from_str::<Value>(&raw).map_err(|err| ParameterError::ParseError {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;

    if !value.is_object() {
        return Err(ParameterError::ParseError {
            path: path.to_path_buf(),
            message: "top-level parameter value must be an object".to_string(),
        });
    }

    Ok(value)
}
