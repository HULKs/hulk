use std::io::ErrorKind;

use color_eyre::{
    eyre::{bail, eyre, Context},
    Result,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use itertools::intersperse;
use tokio::{
    fs::{read_dir, read_link, remove_file, symlink, try_exists},
    io,
};

use crate::Repository;

impl Repository {
    pub async fn list_configured_locations(&self) -> Result<Vec<(String, Option<String>)>> {
        let parameters_root = &self.root.join("etc/parameters");
        let results: Vec<_> = [
            "nao_location",
            "webots_location",
            "behavior_simulator_location",
        ]
        .into_iter()
        .map(|target_name| async move {
            (
                target_name,
                read_link(parameters_root.join(target_name))
                    .await
                    .wrap_err_with(|| format!("failed reading location symlink for {target_name}")),
            )
        })
        .collect::<FuturesUnordered<_>>()
        .collect()
        .await;

        results
            .into_iter()
            .map(|(target_name, path)| match path {
                Ok(path) => Ok((
                    target_name.to_string(),
                    Some(
                        path.file_name()
                            .ok_or_else(|| eyre!("failed to get file name"))?
                            .to_str()
                            .ok_or_else(|| eyre!("failed to convert to UTF-8"))?
                            .to_string(),
                    ),
                )),
                Err(error)
                    if error.downcast_ref::<io::Error>().unwrap().kind() == ErrorKind::NotFound =>
                {
                    Ok((target_name.to_string(), None))
                }
                Err(error) => Err(error),
            })
            .collect()
    }

    pub async fn set_location(&self, target: &str, location: &str) -> Result<()> {
        let parameters_root = self.root.join("etc/parameters");
        if !try_exists(parameters_root.join(location))
            .await
            .wrap_err_with(|| format!("failed checking if location '{location}' exists"))?
        {
            let location_set = self.list_available_locations().await.unwrap_or_default();
            let available_locations: String = intersperse(
                location_set
                    .into_iter()
                    .map(|location| format!("  - {location}")),
                "\n".to_string(),
            )
            .collect();
            bail!(
            "location {location} does not exist.\navailable locations are:\n{available_locations}"
        );
        }
        let target_location = parameters_root.join(format!("{target}_location"));
        let _ = remove_file(&target_location).await;
        symlink(location, &target_location).await.wrap_err_with(|| {
            format!(
                "failed creating symlink named {target_location} pointing to {location}",
                target_location = target_location.display()
            )
        })
    }

    pub async fn list_available_locations(&self) -> Result<Vec<String>> {
        let parameters_root = self.root.join("etc/parameters");
        let mut locations = read_dir(parameters_root)
            .await
            .wrap_err("failed to read parameters directory")?;
        let mut results = Vec::new();
        while let Ok(Some(entry)) = locations.next_entry().await {
            if entry.path().is_dir() && !entry.path().is_symlink() {
                results.push(
                    entry
                        .path()
                        .file_name()
                        .ok_or_else(|| eyre!("failed getting file name for location"))?
                        .to_str()
                        .ok_or_else(|| eyre!("failed to convert to UTF-8"))?
                        .to_string(),
                );
            }
        }
        Ok(results)
    }
}
