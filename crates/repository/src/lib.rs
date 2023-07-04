use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    env::current_dir,
    ffi::OsStr,
    fmt::Display,
    fs::Permissions,
    io::{self, ErrorKind},
    os::unix::prelude::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use color_eyre::{
    eyre::{bail, eyre, Context},
    Result,
};
use constants::SDK_VERSION;
use futures_util::{stream::FuturesUnordered, StreamExt};
use glob::glob;
use home::home_dir;
use parameters::{
    directory::{serialize, Id, Location, Scope},
    json::nest_value_at_path,
};
use serde::Deserialize;
use serde_json::{from_slice, from_str, to_string_pretty, to_value, Value};
use tempfile::{tempdir, TempDir};
use tokio::{
    fs::{
        create_dir_all, read_dir, read_link, read_to_string, remove_file, set_permissions, symlink,
        write, File,
    },
    io::AsyncReadExt,
    process::Command,
};

use spl_network_messages::PlayerNumber;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct Repository {
    root: PathBuf,
}

impl Repository {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn crates_directory(&self) -> PathBuf {
        self.root.join("crates")
    }

    pub fn parameters_root(&self) -> PathBuf {
        self.root.join("etc/parameters")
    }

    pub fn find_latest_file(&self, pattern: &str) -> Result<PathBuf> {
        let path = self.root.join(pattern);
        let matching_paths: Vec<_> = glob(
            path.to_str()
                .ok_or_else(|| eyre!("failed to interpret path as Unicode"))?,
        )
        .wrap_err("failed to execute glob() over target directory")?
        .map(|entry| {
            let path = entry.wrap_err("failed to get glob() entry")?;
            let metadata = path
                .metadata()
                .wrap_err_with(|| format!("failed to get metadata of path {path:?}"))?;
            let modified_time = metadata.modified().wrap_err_with(|| {
                format!("failed to get modified time from metadata of path {path:?}")
            })?;
            Ok((path, modified_time))
        })
        .collect::<Result<_>>()
        .wrap_err("failed to get matching paths")?;
        let (path_with_maximal_modified_time, _modified_time) = matching_paths
            .iter()
            .max_by_key(|(_path, modified_time)| modified_time)
            .ok_or_else(|| eyre!("failed to find any matching path"))?;
        Ok(path_with_maximal_modified_time.to_path_buf())
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
                let manifest = format!("crates/hulk_{target}/Cargo.toml");
                format!("--manifest-path={} ", self.root.join(manifest).display())
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
            .wrap_err("failed to execute cargo command")?;

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

    pub async fn set_player_number(
        &self,
        head_id: &str,
        player_number: PlayerNumber,
    ) -> Result<()> {
        let path = "player_number";
        let parameters = nest_value_at_path(
            path,
            to_value(player_number).wrap_err("failed to serialize player number")?,
        );
        serialize(
            &parameters,
            Scope {
                location: Location::All,
                id: Id::Head,
            },
            path,
            self.parameters_root(),
            "unknown_body_id",
            head_id,
        )
        .await
        .wrap_err("failed to serialize parameters directory")
    }

    pub async fn set_communication(&self, enable: bool) -> Result<()> {
        let file_contents = read_to_string(self.root.join("etc/parameters/hardware.json"))
            .await
            .wrap_err("failed to read hardware.json")?;
        let mut hardware_json: Value =
            from_str(&file_contents).wrap_err("failed to deserialize hardware.json")?;

        hardware_json["communication_addresses"] = if enable {
            Value::String("[::]:1337".to_string())
        } else {
            Value::Null
        };
        {
            let file_contents = to_string_pretty(&hardware_json)
                .wrap_err("failed to serialize hardware.json")?
                + "\n";
            write(
                self.root.join("etc/parameters/hardware.json"),
                file_contents.as_bytes(),
            )
            .await
            .wrap_err("failed to write hardware.json")?;
        }
        Ok(())
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
                .ok_or_else(|| eyre!("cannot find HOME directory"))?
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
                download_sdk(&downloads_directory, version, &installer_name)
                    .await
                    .wrap_err("failed to download SDK")?;
            }
            install_sdk(installer_path, &sdk)
                .await
                .wrap_err("Failed to install SDK")?;
        }
        Ok(())
    }

    pub async fn create_upload_directory(&self, profile: &str) -> Result<(TempDir, PathBuf)> {
        let upload_directory = tempdir().wrap_err("failed to create temporary directory")?;
        let hulk_directory = upload_directory.path().join("hulk");

        // the target directory is "debug" with --profile dev...
        let profile_directory = match profile {
            "dev" => "debug",
            other => other,
        };

        create_dir_all(hulk_directory.join("bin"))
            .await
            .wrap_err("failed to create directory")?;

        symlink(self.root.join("etc"), hulk_directory.join("etc"))
            .await
            .wrap_err("failed to link etc directory")?;

        symlink(
            self.root.join(format!(
                "target/x86_64-aldebaran-linux-gnu/{profile_directory}/hulk_nao"
            )),
            hulk_directory.join("bin/hulk"),
        )
        .await
        .wrap_err("failed to link executable")?;

        Ok((upload_directory, hulk_directory))
    }

    pub async fn get_hardware_ids(&self) -> Result<HashMap<u8, HardwareIds>> {
        let hardware_ids_path = self.root.join("etc/parameters/hardware_ids.json");
        let mut hardware_ids = File::open(&hardware_ids_path)
            .await
            .wrap_err_with(|| format!("Failed to open {}", hardware_ids_path.display()))?;
        let mut contents = vec![];
        hardware_ids.read_to_end(&mut contents).await?;
        let hardware_ids_with_string_keys: HashMap<String, HardwareIds> = from_slice(&contents)?;
        let hardware_ids_with_nao_number_keys = hardware_ids_with_string_keys
            .into_iter()
            .map(|(nao_number, hardware_ids)| {
                Ok((
                    nao_number
                        .parse()
                        .wrap_err_with(|| format!("Failed to parse NAO number: {nao_number:?}"))?,
                    hardware_ids,
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        Ok(hardware_ids_with_nao_number_keys)
    }

    pub async fn get_configured_locations(&self) -> Result<BTreeMap<String, Option<String>>> {
        let results: Vec<_> = [
            "nao_location",
            "webots_location",
            "behavior_simulator_location",
        ]
        .into_iter()
        .map(|target_name| async move {
            (
                target_name,
                read_link(self.parameters_root().join(target_name))
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
        let target_location = self.parameters_root().join(format!("{target}_location"));
        let new_location = self.parameters_root().join(location);
        let _ = remove_file(&target_location).await;
        symlink(&new_location, &target_location)
            .await
            .wrap_err_with(|| {
                format!("failed creating symlink from {new_location:?} to {target_location:?}, does the location exist?"
                )
            })
    }

    pub async fn list_available_locations(&self) -> Result<BTreeSet<String>> {
        let parameters_path = self.root.join("etc/parameters");
        let mut locations = read_dir(parameters_path)
            .await
            .wrap_err("failed parameters root")?;
        let mut results = BTreeSet::new();
        while let Ok(Some(entry)) = locations.next_entry().await {
            if entry.path().is_dir() && !entry.path().is_symlink() {
                results.insert(
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

async fn download_with_fallback(
    output_path: impl AsRef<OsStr>,
    urls: impl IntoIterator<Item = impl AsRef<OsStr>>,
    connect_timeout: Duration,
) -> Result<()> {
    for (i, url) in urls.into_iter().enumerate() {
        let url = url.as_ref();
        if i > 0 {
            println!("Falling back to downloading from {url:?}");
        }

        let status = Command::new("curl")
            .arg("--connect-timeout")
            .arg(connect_timeout.as_secs_f32().to_string())
            .arg("--fail")
            .arg("--location")
            .arg("--progress-bar")
            .arg("--output")
            .arg(&output_path)
            .arg(url)
            .status()
            .await
            .context("Failed to spawn command")?;

        if status.success() {
            return Ok(());
        }
    }

    bail!("curl exited with error")
}

async fn download_image(
    downloads_directory: impl AsRef<Path>,
    version: &str,
    image_name: &str,
) -> Result<()> {
    if !downloads_directory.as_ref().exists() {
        create_dir_all(&downloads_directory)
            .await
            .context("Failed to create download directory")?;
    }
    let image_path = downloads_directory.as_ref().join(image_name);
    let urls = [
        format!("http://bighulk.hulks.dev/image/{image_name}"),
        format!("https://github.com/HULKs/meta-hulks/releases/download/{version}/{image_name}"),
    ];

    println!("Downloading image from {}", urls[0]);
    download_with_fallback(&image_path, urls, CONNECT_TIMEOUT).await
}

pub async fn get_image_path(version: &str) -> Result<PathBuf> {
    let downloads_directory = home_dir()
        .ok_or_else(|| eyre!("cannot find HOME directory"))?
        .join(".naosdk/images");
    let image_name = format!("nao-image-HULKs-OS-{version}.ext3.gz.opn");
    let image_path = downloads_directory.join(&image_name);

    if !image_path.exists() {
        download_image(downloads_directory, version, &image_name).await?;
    }
    Ok(image_path)
}

async fn download_sdk(
    downloads_directory: impl AsRef<Path>,
    version: &str,
    installer_name: &str,
) -> Result<()> {
    if !downloads_directory.as_ref().exists() {
        create_dir_all(&downloads_directory)
            .await
            .context("Failed to create download directory")?;
    }
    let installer_path = downloads_directory.as_ref().join(installer_name);
    let urls = [
        format!("http://bighulk.hulks.dev/sdk/{installer_name}"),
        format!("https://github.com/HULKs/meta-hulks/releases/download/{version}/{installer_name}"),
    ];

    println!("Downloading SDK from {}", urls[0]);
    download_with_fallback(&installer_path, urls, CONNECT_TIMEOUT).await?;

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

pub async fn get_repository_root() -> Result<PathBuf> {
    let path = current_dir().wrap_err("failed to get current directory")?;
    let ancestors = path.as_path().ancestors();
    for ancestor in ancestors {
        let mut directory = read_dir(ancestor)
            .await
            .wrap_err_with(|| format!("failed to read directory {ancestor:?}"))?;
        while let Some(child) = directory.next_entry().await.wrap_err_with(|| {
            format!("failed to get next directory entry while iterating {ancestor:?}")
        })? {
            if child.file_name() == ".git" {
                return Ok(child
                    .path()
                    .parent()
                    .ok_or_else(|| eyre!("failed to get parent of {child:?}"))?
                    .to_path_buf());
            }
        }
    }

    bail!("failed to find .git directory")
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

#[derive(Clone, Debug, Deserialize)]
pub struct HardwareIds {
    pub body_id: String,
    pub head_id: String,
}
