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

#[cfg(test)]
mod tests {
    use super::load_json5_object_or_empty;

    #[test]
    fn parameter_parse_error_preserves_source() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.json5");
        std::fs::write(&path, "{ not valid json5 ").unwrap();

        let error = load_json5_object_or_empty(&path).expect_err("invalid json5 should fail");

        assert!(error.to_string().contains("failed to parse parameter file"));
        assert!(std::error::Error::source(&error).is_some());
    }
}
