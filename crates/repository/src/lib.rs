use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Display,
    fs::Permissions,
    io::{self, ErrorKind},
    os::unix::prelude::PermissionsExt,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use futures::future::join_all;
use home::home_dir;
use serde::Deserialize;
use serde_json::{from_slice, to_value, to_vec_pretty, Value};
use tempfile::{tempdir, TempDir};
use tokio::{
    fs::{
        create_dir_all, read_dir, read_link, remove_file, set_permissions, symlink, File,
        OpenOptions,
    },
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

use spl_network::PlayerNumber;

pub const SDK_VERSION: &str = "5.0";

pub struct Repository {
    root: PathBuf,
}

impl Repository {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    async fn cargo(
        &self,
        action: CargoAction,
        workspace: bool,
        profile: &str,
        target: &str,
        passthrough_arguments: &[String],
    ) -> Result<()> {
        let mut shell_command = String::new();

        if target == "nao" {
            shell_command += &format!(
                ". {} && ",
                self.root
                    .join(format!(
                        "naosdk/{SDK_VERSION}/environment-setup-corei7-64-aldebaran-linux"
                    ))
                    .display()
            );
        }

        let cargo_command = format!("cargo {action} ")
            + format!("--profile {profile} ").as_str()
            + if workspace {
                "--workspace --all-features --all-targets ".to_string()
            } else {
                format!("--features {target} --bin {target} ")
            }
            .as_str()
            + "-- "
            + match action {
                CargoAction::Clippy => "--deny warnings ",
                _ => "",
            }
            + passthrough_arguments.join(" ").as_str();

        println!("Running: {cargo_command}");

        let status = Command::new("sh")
            .arg("-c")
            .arg(shell_command + &cargo_command)
            .status()
            .await
            .context("Failed to execute cargo command")?;

        if !status.success() {
            bail!("cargo command exited with {status}");
        }

        Ok(())
    }

    pub async fn build(
        &self,
        workspace: bool,
        profile: &str,
        target: &str,
        passthrough_arguments: &[String],
    ) -> Result<()> {
        self.cargo(
            CargoAction::Build,
            workspace,
            profile,
            target,
            passthrough_arguments,
        )
        .await
    }

    pub async fn check(&self, workspace: bool, profile: &str, target: &str) -> Result<()> {
        self.cargo(CargoAction::Check, workspace, profile, target, &[])
            .await
    }

    pub async fn clippy(&self, workspace: bool, profile: &str, target: &str) -> Result<()> {
        self.cargo(CargoAction::Clippy, workspace, profile, target, &[])
            .await
    }

    pub async fn run(
        &self,
        profile: &str,
        target: &str,
        passthrough_arguments: &[String],
    ) -> Result<()> {
        self.cargo(
            CargoAction::Run,
            false,
            profile,
            target,
            passthrough_arguments,
        )
        .await
    }

    fn configuration_root(&self) -> PathBuf {
        self.root.join("etc/configuration")
    }

    fn head_configuration(&self, head_id: &str) -> PathBuf {
        self.configuration_root()
            .join(format!("head.{head_id}.json"))
    }

    async fn read_configuration(&self, head_id: &str) -> Result<Value> {
        let configuration_file_path = self.head_configuration(head_id);
        let mut configuration_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&configuration_file_path)
            .await
            .with_context(|| format!("Failed to open {}", configuration_file_path.display()))?;

        let mut contents = vec![];
        configuration_file
            .read_to_end(&mut contents)
            .await
            .with_context(|| {
                format!("Failed to read from {}", configuration_file_path.display())
            })?;
        Ok(if contents.is_empty() {
            Value::Object(Default::default())
        } else {
            from_slice(&contents)
                .with_context(|| format!("Failed to parse {}", configuration_file_path.display()))?
        })
    }

    async fn write_configuration(&self, head_id: &str, configuration: &Value) -> Result<()> {
        let configuration_file_path = self.head_configuration(head_id);
        let mut contents = to_vec_pretty(configuration).with_context(|| {
            format!(
                "Failed to dump configuration for {}",
                configuration_file_path.display()
            )
        })?;
        contents.push(b'\n');
        let mut configuration_file = File::create(&configuration_file_path)
            .await
            .with_context(|| format!("Failed to create {}", configuration_file_path.display()))?;
        configuration_file
            .write_all(&contents)
            .await
            .with_context(|| format!("Failed to parse {}", configuration_file_path.display()))?;
        Ok(())
    }

    pub async fn set_player_number(
        &self,
        head_id: &str,
        player_number: PlayerNumber,
    ) -> Result<()> {
        let mut configuration = self
            .read_configuration(head_id)
            .await
            .context("Failed to read configuration")?;

        configuration["player_number"] =
            to_value(player_number).context("Failed to serialize player number")?;

        self.write_configuration(head_id, &configuration)
            .await
            .context("Failed to write configuration")
    }

    pub async fn set_communication(&self, head_id: &str, enable: bool) -> Result<()> {
        let mut configuration = self
            .read_configuration(head_id)
            .await
            .context("Failed to read configuration")?;

        if enable {
            if let Value::Object(ref mut object) = configuration {
                object.remove("disable_communication_acceptor");
            }
        } else {
            configuration["disable_communication_acceptor"] = Value::Bool(true);
        }

        self.write_configuration(head_id, &configuration)
            .await
            .context("Failed to write configuration")
    }

    pub async fn install_sdk(
        &self,
        version: Option<&str>,
        installation_directory: Option<&Path>,
    ) -> Result<()> {
        let symlink = self.root.join("naosdk");
        let version = version.unwrap_or(SDK_VERSION);
        let installation_directory = if let Some(directory) = installation_directory {
            create_symlink(directory, &symlink).await?;
            directory.to_path_buf()
        } else if symlink.exists() {
            symlink.clone()
        } else {
            let directory = home_dir()
                .context("Cannot find HOME directory")?
                .join(".naosdk");
            create_symlink(&directory, &symlink).await?;
            directory
        };
        let sdk = installation_directory.join(version);
        if !sdk.exists() {
            let downloads_directory = installation_directory.join("downloads");
            let installer_name = format!("HULKs-OS-toolchain-{version}.sh");
            let installer_path = downloads_directory.join(&installer_name);
            if !installer_path.exists() {
                download_sdk(&downloads_directory, &installer_name)
                    .await
                    .context("Failed to download SDK")?;
            }
            install_sdk(installer_path, &sdk)
                .await
                .context("Failed to install SDK")?;
        }
        Ok(())
    }

    pub async fn create_upload_directory(&self, profile: String) -> Result<(TempDir, PathBuf)> {
        let upload_directory = tempdir().context("Failed to create temporary directory")?;
        let hulk_directory = upload_directory.path().join("hulk");

        create_dir_all(hulk_directory.join("bin"))
            .await
            .context("Failed to create directory")?;

        symlink(self.root.join("etc"), hulk_directory.join("etc"))
            .await
            .context("Failed to link etc directory")?;

        symlink(
            self.root
                .join(format!("target/x86_64-aldebaran-linux-gnu/{profile}/nao")),
            hulk_directory.join("bin/hulk"),
        )
        .await
        .context("Failed to link executable")?;

        Ok((upload_directory, hulk_directory))
    }

    pub async fn get_hardware_ids(&self) -> Result<HashMap<u8, HardwareIds>> {
        let hardware_ids_path = self.root.join("etc/configuration/hardware_ids.json");
        let mut hardware_ids = File::open(&hardware_ids_path)
            .await
            .with_context(|| format!("Failed to open {}", hardware_ids_path.display()))?;
        let mut contents = vec![];
        hardware_ids.read_to_end(&mut contents).await?;
        let hardware_ids_with_string_keys: HashMap<String, HardwareIds> = from_slice(&contents)?;
        let hardware_ids_with_nao_number_keys = hardware_ids_with_string_keys
            .into_iter()
            .map(|(nao_number, hardware_ids)| {
                Ok((
                    nao_number
                        .parse()
                        .with_context(|| format!("Failed to parse NAO number: {:?}", nao_number))?,
                    hardware_ids,
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        Ok(hardware_ids_with_nao_number_keys)
    }

    pub async fn get_configured_locations(&self) -> Result<BTreeMap<String, Option<String>>> {
        let tasks = ["nao_location", "webots_location", "behavior_simulator"]
            .into_iter()
            .map(|target_name| async move {
                (
                    target_name,
                    read_link(self.configuration_root().join(target_name))
                        .await
                        .with_context(|| {
                            anyhow!("Failed reading location symlink for {target_name}")
                        }),
                )
            });
        let results = join_all(tasks).await;
        results
            .into_iter()
            .map(|(target_name, path)| match path {
                Ok(path) => Ok((
                    target_name.to_string(),
                    Some(
                        path.file_name()
                            .ok_or_else(|| anyhow!("Failed to get file name"))?
                            .to_str()
                            .ok_or_else(|| anyhow!("Failed to convert to UTF-8"))?
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
        let target_location = self.configuration_root().join(target);
        let new_location = self.configuration_root().join(location);
        remove_file(&target_location)
            .await
            .with_context(|| anyhow!("Failed removing symlink for {target_location:?}"))?;
        symlink(&new_location, &target_location)
            .await
            .with_context(|| {
                anyhow!(
                    "Failed creating symlink from {new_location:?} to {target_location:?}, does the location exist?"
                )
            })
    }

    pub async fn list_available_locations(&self) -> Result<BTreeSet<String>> {
        let configuration_path = self.root.join("etc/configuration");
        let mut locations = read_dir(configuration_path)
            .await
            .context("Failed configuration root")?;
        let mut results = BTreeSet::new();
        while let Ok(Some(entry)) = locations.next_entry().await {
            if entry.path().is_dir() && !entry.path().is_symlink() {
                results.insert(
                    entry
                        .path()
                        .file_name()
                        .with_context(|| anyhow!("Failed getting file name for location"))?
                        .to_str()
                        .with_context(|| anyhow!("Failed to convert to UTF-8"))?
                        .to_string(),
                );
            }
        }
        Ok(results)
    }
}

async fn download_sdk(downloads_directory: impl AsRef<Path>, installer_name: &str) -> Result<()> {
    if !downloads_directory.as_ref().exists() {
        create_dir_all(&downloads_directory)
            .await
            .context("Failed to create download directory")?;
    }
    let installer_path = downloads_directory.as_ref().join(installer_name);
    let url = format!("http://bighulk.hulks.dev/sdk/{installer_name}");
    println!("Downloading SDK from {url}");
    let status = Command::new("curl")
        .arg("--progress-bar")
        .arg("--output")
        .arg(&installer_path)
        .arg(url)
        .status()
        .await
        .context("Failed to spawn command")?;

    if !status.success() {
        bail!("curl exited with {status}");
    }

    set_permissions(&installer_path, Permissions::from_mode(0o755))
        .await
        .context("Failed to make installer executable")
}

async fn install_sdk(
    installer_path: impl AsRef<Path>,
    installation_directory: impl AsRef<Path>,
) -> Result<()> {
    let status = Command::new(installer_path.as_ref().as_os_str())
        .arg("-d")
        .arg(installation_directory.as_ref().as_os_str())
        .status()
        .await
        .context("Failed to spawn command")?;

    if !status.success() {
        bail!("SDK installer exited with {status}");
    }
    Ok(())
}

async fn create_symlink(source: &Path, destination: &Path) -> Result<()> {
    if destination.read_link().is_ok() {
        remove_file(&destination)
            .await
            .context("Failed to remove current symlink")?;
    }
    symlink(&source, &destination)
        .await
        .context("Failed to create symlink")?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum CargoAction {
    Build,
    Check,
    Clippy,
    Run,
}

impl Display for CargoAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CargoAction::Build => "build",
                CargoAction::Check => "check",
                CargoAction::Clippy => "clippy",
                CargoAction::Run => "run",
            }
        )
    }
}

#[derive(Deserialize)]
pub struct HardwareIds {
    pub body_id: String,
    pub head_id: String,
}
