use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub fn rust_file_paths_from<P>(parent_directory: P) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    let all_entries = WalkDir::new(parent_directory).into_iter();
    let only_ok_entries = all_entries.filter_map(|entry| entry.ok());
    let only_files = only_ok_entries.filter(|entry| {
        matches!(entry.metadata().ok(),
        Some(metadata) if metadata.is_file())
    });
    let only_rs_files = only_files.filter(|entry| {
        entry
            .path()
            .extension()
            .map_or(false, |extension| extension == "rs")
    });
    only_rs_files
        .map(|entry| entry.path().to_path_buf())
        .collect()
}
