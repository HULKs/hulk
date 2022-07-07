use std::{
    collections::HashMap,
    env::var_os,
    fmt::Display,
    fs::Permissions,
    io::ErrorKind,
    os::unix::prelude::PermissionsExt,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use serde::Deserialize;
use serde_json::{from_slice, to_value, to_vec, Value};
use spl_network::PlayerNumber;
use tempfile::{tempdir, TempDir};
use tokio::{
    fs::{create_dir, create_dir_all, remove_file, set_permissions, symlink, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

pub const SDK_VERSION: &str = "4.2";
pub const INSTALLATION_DIRECTORY: &str = "/opt/nao";

pub struct Repository {
    root: PathBuf,
}

impl Repository {
    pub fn new<P>(root: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn get_private_key_path(&self) -> PathBuf {
        self.root.join("scripts/ssh_key")
    }

    pub async fn fix_private_key_permissions(&self) -> anyhow::Result<()> {
        let private_key = self.get_private_key_path();
        let metadata = private_key
            .metadata()
            .context("Failed to get metadata of SSH key")?;
        let mut permissions = metadata.permissions();
        let read_write_for_owner_only = 0o600;
        let permission_bits = extract_permission_bits(permissions.mode());
        if permission_bits != read_write_for_owner_only {
            permissions.set_mode(compose_mode(permissions.mode(), read_write_for_owner_only));
            set_permissions(private_key, permissions)
                .await
                .context("Failed to set permissions on SSH key")?;
        }

        Ok(())
    }

    async fn cargo(
        &self,
        action: CargoAction,
        profile: String,
        target: String,
    ) -> anyhow::Result<()> {
        let mut command = Command::new("sh");

        let mut command_string = String::new();

        if target == "nao" {
            command_string += format!(
                ". {:?} && ",
                self.root
                    .join("sdk/current/environment-setup-corei7-64-aldebaran-linux")
            )
            .as_str();
            let nao_cargo_home = var_os("NAO_CARGO_HOME")
                .unwrap_or_else(|| self.root.join(".nao_cargo_home").into_os_string());
            command.env("NAO_CARGO_HOME", nao_cargo_home);
        }

        let cargo_command =
            format!("cargo {action} --profile {profile} --features {target} --bin {target}");
        command_string += &cargo_command;
        command.arg("-c").arg(command_string);

        let status = command
            .status()
            .await
            .context("Failed to execute cargo command")?;

        if !status.success() {
            bail!("cargo command exited with {status}");
        }

        Ok(())
    }

    pub async fn build(&self, profile: String, target: String) -> anyhow::Result<()> {
        self.cargo(CargoAction::Build, profile, target).await
    }

    pub async fn check(&self, profile: String, target: String) -> anyhow::Result<()> {
        self.cargo(CargoAction::Check, profile, target).await
    }

    pub async fn run(&self, profile: String, target: String) -> anyhow::Result<()> {
        self.cargo(CargoAction::Run, profile, target).await
    }

    fn get_configuration_path(&self, head_id: &str) -> PathBuf {
        self.root
            .join(format!("etc/configuration/head.{}.json", head_id))
    }

    async fn read_configuration(&self, head_id: &str) -> anyhow::Result<Value> {
        let configuration_file_path = self.get_configuration_path(head_id);
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

    async fn write_configuration(&self, head_id: &str, configuration: Value) -> anyhow::Result<()> {
        let configuration_file_path = self.get_configuration_path(head_id);
        let mut contents = to_vec(&configuration).with_context(|| {
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

        let status = match Command::new("prettier")
            .arg("--write")
            .arg("--loglevel=warn")
            .arg(&configuration_file_path)
            .status()
            .await
        {
            Ok(status) => status,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Err(error).context("prettier command not found, is it installed?")?
            }
            Err(error) => Err(error).context("Failed to execute prettier command")?,
        };

        if !status.success() {
            bail!("prettier command exited with {status}");
        }

        Ok(())
    }

    pub async fn set_player_number(
        &self,
        head_id: &str,
        player_number: PlayerNumber,
    ) -> anyhow::Result<()> {
        let mut configuration = self
            .read_configuration(head_id)
            .await
            .context("Failed to read configuration")?;

        configuration["player_number"] =
            to_value(player_number).context("Failed to serialize player number")?;

        self.write_configuration(head_id, configuration)
            .await
            .context("Failed to write configuration")
    }

    pub async fn set_communication(&self, head_id: &str, enable: bool) -> anyhow::Result<()> {
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

        self.write_configuration(head_id, configuration)
            .await
            .context("Failed to write configuration")
    }

    pub async fn install_sdk(
        &self,
        force_reinstall: bool,
        alternative_sdk_version: Option<String>,
        alternative_installation_directory: Option<PathBuf>,
    ) -> anyhow::Result<()> {
        let sdk_directory = self.root.join("sdk");
        let sdk_version = alternative_sdk_version.unwrap_or_else(|| SDK_VERSION.to_string());
        let installation_directory =
            alternative_installation_directory.unwrap_or_else(|| INSTALLATION_DIRECTORY.into());
        let installation_directory = installation_directory.join(&sdk_version);
        let needs_installation = force_reinstall || !installation_directory.exists();
        if !needs_installation {
            let current_symlink = sdk_directory.join("current");
            if !current_symlink.exists() {
                Self::create_sdk_symlink(&sdk_directory, &installation_directory).await?;
            }
            return Ok(());
        }

        let downloads_directory = sdk_directory.join("downloads");
        let installer_name = format!("HULKs-OS-toolchain-{}.sh", sdk_version);
        let download_file_path = downloads_directory.join(&installer_name);
        if !download_file_path.exists() {
            if !downloads_directory.exists() {
                create_dir(downloads_directory)
                    .await
                    .context("Failed to create download directory")?;
            }
            let url = format!("http://bighulk/sdk/{}", installer_name);
            let status = Command::new("curl")
                .arg("--progress-bar")
                .arg("--output")
                .arg(&download_file_path)
                .arg(url)
                .status()
                .await
                .context("Failed to download SDK")?;

            if !status.success() {
                bail!("curl exited with {}", status);
            }

            set_permissions(&download_file_path, Permissions::from_mode(0o755))
                .await
                .context("Failed to make installer executable")?;
        }

        let status = Command::new(download_file_path)
            .arg("-d")
            .arg(&installation_directory)
            .status()
            .await
            .context("Failed to install SDK")?;

        if !status.success() {
            bail!("SDK installer exited with {}", status);
        }

        Self::create_sdk_symlink(&sdk_directory, &installation_directory).await?;

        Ok(())
    }

    async fn create_sdk_symlink(
        sdk_directory: &Path,
        installation_directory: &Path,
    ) -> anyhow::Result<()> {
        let symlink_path = sdk_directory.join("current");
        if symlink_path.read_link().is_ok() {
            remove_file(&symlink_path)
                .await
                .context("Failed to remove current SDK symlink")?;
        }
        symlink(&installation_directory, &symlink_path)
            .await
            .context("Failed to symlink current SDK to installation directory")?;
        Ok(())
    }

    pub async fn create_upload_directory(
        &self,
        profile: String,
    ) -> anyhow::Result<(TempDir, PathBuf)> {
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
                .join(format!("target/x86_64-aldebaran-linux/{}/nao", profile)),
            hulk_directory.join("bin/hulk"),
        )
        .await
        .context("Failed to link executable")?;

        Ok((upload_directory, hulk_directory))
    }

    pub async fn get_hardware_ids(&self) -> anyhow::Result<HashMap<u8, HardwareIds>> {
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
            .collect::<anyhow::Result<HashMap<_, _>>>()?;
        Ok(hardware_ids_with_nao_number_keys)
    }
}

enum CargoAction {
    Build,
    Check,
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

fn extract_permission_bits(mode: u32) -> u32 {
    mode & PERMISSION_BITS_MASK
}

fn compose_mode(old_mode: u32, new_permissions: u32) -> u32 {
    (old_mode & !PERMISSION_BITS_MASK) | (new_permissions & PERMISSION_BITS_MASK)
}

#[allow(clippy::unusual_byte_groupings)]
const PERMISSION_BITS_MASK: u32 = 0b111_111_111;
