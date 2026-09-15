use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use color_eyre::{
    Result,
    eyre::{ContextCompat as _, WrapErr as _, ensure},
};

use crate::layout::TwixLayout;

pub const PROVIDED: &[(&str, &str)] = &[
    ("Overview", include_str!("../presets/Overview.json")),
    ("Vision", include_str!("../presets/Vision.json")),
    ("Parameters", include_str!("../presets/Parameters.json")),
];

pub fn directory() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .wrap_err("configuration directory unavailable")?
        .join("hulks/twix-ros-z/presets"))
}

pub fn list(directory: &Path) -> Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().extension().is_some_and(|ext| ext == "json")
        {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn path(directory: &Path, name: &str) -> Result<PathBuf> {
    ensure!(
        !name.trim().is_empty()
            && name == name.trim()
            && !name
                .chars()
                .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
            && name != "."
            && name != "..",
        "enter a name without path separators or special characters"
    );
    Ok(directory.join(format!("{name}.json")))
}

pub fn save(path: &Path, serialized: &str) -> Result<()> {
    TwixLayout::validate(serialized)?;
    let directory = path.parent().wrap_err("preset has no parent directory")?;
    fs::create_dir_all(directory)?;
    let temporary = directory.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::File::create_new(&temporary)?;
        file.write_all(serialized.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.wrap_err_with(|| format!("failed to save {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_bundled_presets_are_registered_and_valid() {
        let files = list(&Path::new(env!("CARGO_MANIFEST_DIR")).join("presets")).unwrap();
        assert_eq!(files.len(), PROVIDED.len());
        for &(name, contents) in PROVIDED {
            let file = files
                .iter()
                .find(|path| path.file_stem().unwrap() == name)
                .expect("registered preset file");
            assert_eq!(fs::read_to_string(file).unwrap(), contents);
            TwixLayout::validate(contents).unwrap_or_else(|error| panic!("{name}: {error:#}"));
        }
    }

    #[test]
    fn user_presets_save_overwrite_and_validate_names() {
        let directory = std::env::temp_dir().join(format!("twix-presets-{}", uuid::Uuid::new_v4()));
        assert!(list(&directory).unwrap().is_empty());
        for name in ["", " ", "..", "../escape", "a/b", "a\\b", "trailing "] {
            assert!(path(&directory, name).is_err());
        }
        let file = path(&directory, "My layout").unwrap();
        save(&file, PROVIDED[0].1).unwrap();
        save(&file, PROVIDED[1].1).unwrap();
        assert!(save(&file, "{}").is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), PROVIDED[1].1);
        fs::create_dir(directory.join("directory.json")).unwrap();
        fs::write(directory.join("ignored.tmp"), "{}").unwrap();
        assert_eq!(list(&directory).unwrap(), vec![file]);
        fs::remove_dir_all(directory).unwrap();
    }
}
