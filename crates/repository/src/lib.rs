pub mod cargo;
pub mod communication;
pub mod configuration;
pub mod data_home;
pub mod download;
pub mod find_root;
pub mod hardware_ids;
pub mod image;
pub mod inspect_version;
pub mod location;
pub mod modify_json;
pub mod player_number;
pub mod recording;
pub mod sdk;
pub mod symlink;

//async fn cargo(
//    &self,
//    action: CargoAction,
//    workspace: bool,
//    profile: &str,
//    target: &str,
//    features: Option<Vec<String>>,
//    passthrough_arguments: &[String],
//) -> Result<()> {
//    let os_is_not_linux = !cfg!(target_os = "linux");
//    let use_docker = target == "nao" && os_is_not_linux;
//
//    let cargo_command = format!("cargo {action} ")
//        + format!("--profile {profile} ").as_str()
//        + if let Some(features) = features {
//            let features = features.join(",");
//            format!("--features {features} ")
//        } else {
//            String::new()
//        }
//        .as_str()
//        + if workspace {
//            "--workspace --all-features --all-targets ".to_string()
//        } else {
//            let manifest = format!("crates/hulk_{target}/Cargo.toml");
//            let root = if use_docker {
//                Path::new("/hulk")
//            } else {
//                &self.root
//            };
//            format!("--manifest-path={} ", root.join(manifest).display())
//        }
//        .as_str()
//        + "-- "
//        + match action {
//            CargoAction::Clippy => "--deny warnings ",
//            _ => "",
//        }
//        + passthrough_arguments.join(" ").as_str();
//
//    println!("Running: {cargo_command}");
//
//    let shell_command = if use_docker {
//        format!(
//                "docker run --volume={}:/hulk --volume={}:/naosdk/sysroots/corei7-64-aldebaran-linux/home/cargo \
//                --rm --interactive --tty ghcr.io/hulks/naosdk:{SDK_VERSION} /bin/bash -c \
//                '. /naosdk/environment-setup-corei7-64-aldebaran-linux && {cargo_command}'",
//                self.root.display(),
//                self.root.join("naosdk/cargo-home").join(SDK_VERSION).display()
//            )
//    } else if target == "nao" {
//        format!(
//            ". {} && {cargo_command}",
//            self.root
//                .join(format!(
//                    "naosdk/{SDK_VERSION}/environment-setup-corei7-64-aldebaran-linux"
//                ))
//                .display()
//        )
//    } else {
//        cargo_command
//    };
//
//    let status = Command::new("sh")
//        .arg("-c")
//        .arg(shell_command)
//        .status()
//        .await
//        .wrap_err("failed to execute cargo command")?;
//
//    if !status.success() {
//        bail!("cargo command exited with {status}");
//    }
//
//    Ok(())
//}
//
//pub async fn build(
//    &self,
//    workspace: bool,
//    profile: &str,
//    target: &str,
//    features: Option<Vec<String>>,
//    passthrough_arguments: &[String],
//) -> Result<()> {
//    self.cargo(
//        CargoAction::Build,
//        workspace,
//        profile,
//        target,
//        features,
//        passthrough_arguments,
//    )
//    .await
//}
//
//pub async fn check(&self, workspace: bool, profile: &str, target: &str) -> Result<()> {
//    self.cargo(CargoAction::Check, workspace, profile, target, None, &[])
//        .await
//}
//
//pub async fn clippy(&self, workspace: bool, profile: &str, target: &str) -> Result<()> {
//    self.cargo(CargoAction::Clippy, workspace, profile, target, None, &[])
//        .await
//}
//
//pub async fn run(
//    &self,
//    profile: &str,
//    target: &str,
//    features: Option<Vec<String>>,
//    passthrough_arguments: &[String],
//) -> Result<()> {
//    self.cargo(
//        CargoAction::Run,
//        false,
//        profile,
//        target,
//        features,
//        passthrough_arguments,
//    )
//    .await
//}
//
//pub async fn create_upload_directory(&self, profile: &str) -> Result<(TempDir, PathBuf)> {
//    let upload_directory = tempdir().wrap_err("failed to create temporary directory")?;
//    let hulk_directory = upload_directory.path().join("hulk");
//
//    // the target directory is "debug" with --profile dev...
//    let profile_directory = match profile {
//        "dev" => "debug",
//        other => other,
//    };
//
//    create_dir_all(hulk_directory.join("bin"))
//        .await
//        .wrap_err("failed to create directory")?;
//
//    symlink(self.root.join("etc"), hulk_directory.join("etc"))
//        .await
//        .wrap_err("failed to link etc directory")?;
//
//    symlink(
//        self.root.join(format!(
//            "target/x86_64-aldebaran-linux-gnu/{profile_directory}/hulk_nao"
//        )),
//        hulk_directory.join("bin/hulk"),
//    )
//    .await
//    .wrap_err("failed to link executable")?;
//
//    Ok((upload_directory, hulk_directory))
//}
//
//#[derive(Debug, Clone, Copy)]
//enum CargoAction {
//    Build,
//    Check,
//    Clippy,
//    Run,
//}
//
//impl Display for CargoAction {
//    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//        write!(
//            f,
//            "{}",
//            match self {
//                CargoAction::Build => "build",
//                CargoAction::Check => "check",
//                CargoAction::Clippy => "clippy",
//                CargoAction::Run => "run",
//            }
//        )
//    }
//}
