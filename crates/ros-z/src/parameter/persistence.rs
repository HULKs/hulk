use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde_json::Value;
use tempfile::NamedTempFile;

use super::{ParameterError, Result};

#[cfg(test)]
pub fn write_pretty_json(path: &Path, value: &Value) -> Result<()> {
    let data =
        serde_json::to_string_pretty(value).map_err(|err| ParameterError::PersistenceError {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;

    write_bytes_atomic(path, data.as_bytes())
}

pub fn write_pretty_json_batch(entries: &[(PathBuf, Value)]) -> Result<()> {
    struct PreparedWrite {
        target: PathBuf,
        parent: PathBuf,
        temp_file: NamedTempFile,
        previous: Option<Vec<u8>>,
    }

    let mut prepared = Vec::with_capacity(entries.len());
    for (path, value) in entries {
        let data = serde_json::to_string_pretty(value).map_err(|err| {
            ParameterError::PersistenceError {
                path: path.clone(),
                message: err.to_string(),
            }
        })?;
        let Some(parent) = target_parent(path) else {
            return Err(ParameterError::PersistenceError {
                path: path.clone(),
                message: "target path has no parent directory".to_string(),
            });
        };

        fs::create_dir_all(parent).map_err(|err| ParameterError::PersistenceError {
            path: path.clone(),
            message: err.to_string(),
        })?;
        if path.is_dir() {
            return Err(ParameterError::PersistenceError {
                path: path.clone(),
                message: "target path is a directory".to_string(),
            });
        }

        let previous = if path.exists() {
            Some(
                fs::read(path).map_err(|err| ParameterError::PersistenceError {
                    path: path.clone(),
                    message: err.to_string(),
                })?,
            )
        } else {
            None
        };

        let mut temp_file =
            NamedTempFile::new_in(parent).map_err(|err| ParameterError::PersistenceError {
                path: parent.to_path_buf(),
                message: err.to_string(),
            })?;
        write_prepared_bytes(&mut temp_file, data.as_bytes())?;
        prepared.push(PreparedWrite {
            target: path.clone(),
            parent: parent.to_path_buf(),
            temp_file,
            previous,
        });
    }

    let mut persisted = Vec::new();
    for PreparedWrite {
        target,
        parent,
        temp_file,
        previous,
    } in prepared
    {
        temp_file
            .persist(&target)
            .map_err(|err| rollback_persisted(&persisted, target.clone(), err.error))?;
        persisted.push((target.clone(), previous));
        sync_parent_directory(&parent)
            .map_err(|err| rollback_persisted(&persisted, target.clone(), err))?;
    }

    Ok(())
}

fn write_bytes_atomic(path: &Path, data: &[u8]) -> Result<()> {
    write_bytes_atomic_with(path, data, |temp_file, data| temp_file.write_all(data))
}

fn write_bytes_atomic_with<F>(path: &Path, data: &[u8], write_bytes: F) -> Result<()>
where
    F: FnOnce(&mut NamedTempFile, &[u8]) -> std::io::Result<()>,
{
    let Some(parent) = target_parent(path) else {
        return Err(ParameterError::PersistenceError {
            path: path.to_path_buf(),
            message: "target path has no parent directory".to_string(),
        });
    };

    fs::create_dir_all(parent).map_err(|err| ParameterError::PersistenceError {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;

    let mut temp_file =
        NamedTempFile::new_in(parent).map_err(|err| ParameterError::PersistenceError {
            path: parent.to_path_buf(),
            message: err.to_string(),
        })?;
    write_bytes(&mut temp_file, data).map_err(|err| ParameterError::PersistenceError {
        path: temp_file.path().to_path_buf(),
        message: err.to_string(),
    })?;
    sync_temp_file(&mut temp_file)?;

    temp_file
        .persist(path)
        .map_err(|err| ParameterError::PersistenceError {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
    sync_parent_directory(parent)?;

    Ok(())
}

fn write_prepared_bytes(temp_file: &mut NamedTempFile, data: &[u8]) -> Result<()> {
    temp_file
        .write_all(data)
        .map_err(|err| ParameterError::PersistenceError {
            path: temp_file.path().to_path_buf(),
            message: err.to_string(),
        })?;
    sync_temp_file(temp_file)
}

fn sync_temp_file(temp_file: &mut NamedTempFile) -> Result<()> {
    temp_file
        .flush()
        .map_err(|err| ParameterError::PersistenceError {
            path: temp_file.path().to_path_buf(),
            message: err.to_string(),
        })?;
    temp_file
        .as_file()
        .sync_all()
        .map_err(|err| ParameterError::PersistenceError {
            path: temp_file.path().to_path_buf(),
            message: err.to_string(),
        })
}

fn rollback_persisted<E: std::fmt::Display>(
    persisted: &[(PathBuf, Option<Vec<u8>>)],
    failed_path: PathBuf,
    error: E,
) -> ParameterError {
    let mut rollback_errors = Vec::new();
    for (path, previous) in persisted.iter().rev() {
        let rollback_result = match previous {
            Some(bytes) => write_bytes_atomic(path, bytes),
            None => fs::remove_file(path).map_err(|err| ParameterError::PersistenceError {
                path: path.clone(),
                message: err.to_string(),
            }),
        };
        if let Err(err) = rollback_result {
            rollback_errors.push(err.to_string());
        }
    }

    let mut message = error.to_string();
    if !rollback_errors.is_empty() {
        message.push_str("; rollback also failed: ");
        message.push_str(&rollback_errors.join("; "));
    }
    ParameterError::PersistenceError {
        path: failed_path,
        message,
    }
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> Result<()> {
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|err| ParameterError::PersistenceError {
            path: parent.to_path_buf(),
            message: err.to_string(),
        })
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> Result<()> {
    Ok(())
}

fn target_parent(path: &Path) -> Option<&Path> {
    path.parent().map(|parent| {
        if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        }
    })
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use serde_json::json;

    use super::*;

    fn directory_entries(path: &Path) -> Vec<std::path::PathBuf> {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    #[test]
    fn successful_write_leaves_only_target_file_in_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let target_path = temp_dir.path().join("parameters.json");

        write_pretty_json(&target_path, &json!({ "a": 1 })).unwrap();

        assert_eq!(directory_entries(temp_dir.path()), vec![target_path]);
    }

    #[test]
    fn injected_write_failure_after_temp_file_creation_cleans_up_temp_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let target_path = temp_dir.path().join("parameters.json");

        let error = write_bytes_atomic_with(&target_path, b"{}", |temp_file, _data| {
            temp_file.write_all(b"partial")?;
            Err(io::Error::other("injected write failure"))
        })
        .expect_err("injected write failure should be returned");

        assert!(error.to_string().contains("injected write failure"));
        assert!(directory_entries(temp_dir.path()).is_empty());
    }

    #[test]
    fn persist_failure_cleans_up_temp_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let target_path = temp_dir.path().join("parameters.json");
        fs::create_dir(&target_path).unwrap();

        let error = write_bytes_atomic(&target_path, b"{}").expect_err("persist should fail");

        assert!(error.to_string().contains("parameters.json"));
        assert_eq!(directory_entries(temp_dir.path()), vec![target_path]);
    }

    #[test]
    fn bare_relative_target_uses_current_directory_parent() {
        assert_eq!(
            target_parent(Path::new("parameters.json")).unwrap(),
            Path::new(".")
        );
    }
}
