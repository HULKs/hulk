use std::{
    env::var_os,
    path::{Path, PathBuf},
};

/// Get the repository root directory.
///
/// This function searches for the `hulk.toml` in the start directory and its ancestors.
/// If found, it returns the path to the directory containing the `hulk.toml`. If not, it falls
/// back to the HULK_DEFAULT_ROOT environment variable.
pub fn find_repository_root(start: impl AsRef<Path>) -> Option<PathBuf> {
    let ancestors = start.as_ref().ancestors();
    ancestors
        .filter_map(|ancestor| std::fs::read_dir(ancestor).ok())
        .flatten()
        .find_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_name() == "hulk.toml" {
                Some(entry.path().parent()?.to_path_buf())
            } else {
                None
            }
        })
        .or_else(|| var_os("HULK_DEFAULT_ROOT").map(PathBuf::from))
}
