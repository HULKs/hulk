use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use color_eyre::{
    Result,
    eyre::{ContextCompat, WrapErr, bail, ensure},
};
use regex::Regex;
use repository::Repository;
use serde::{Deserialize, Serialize};
use tokio::{fs, process::Command};

const NIX_INSTALLER_URL: &str =
    "https://releases.nixos.org/nix/nix-2.34.8/nix-2.34.8-aarch64-linux.tar.xz";
const NIX_INSTALLER_SHA256: &str =
    "b4e565fe4db5c352547b8146488cab81db06be89301a4c67b081c76f1e457760";

#[derive(Deserialize, Serialize)]
struct Manifest {
    root: PathBuf,
    #[serde(flatten)]
    runtime: Runtime,
}

#[derive(Deserialize, Serialize)]
struct Runtime {
    runtime_version: String,
    system: String,
    l4t: String,
}

impl Manifest {
    fn validate(&self, version: &str) -> Result<()> {
        ensure!(
            self.runtime.runtime_version == version,
            "runtime bundle version mismatch: expected {version}, found {}",
            self.runtime.runtime_version
        );
        ensure!(
            self.runtime.system == "aarch64-linux" && self.runtime.l4t == "36.4.3",
            "unsupported runtime platform"
        );
        ensure!(
            Regex::new(r"^/nix/store/[0-9a-z]{32}-hulk-runtime$")?
                .is_match(&self.root.to_string_lossy()),
            "invalid runtime root path"
        );
        Ok(())
    }
}

pub(super) async fn validate(bundle: &Path, version: &str) -> Result<PathBuf> {
    let bundle = fs::canonicalize(bundle)
        .await
        .wrap_err("failed to resolve runtime bundle")?;
    let manifest: Manifest = serde_json::from_slice(&fs::read(bundle.join("manifest.json")).await?)
        .wrap_err("invalid runtime bundle manifest")?;
    manifest.validate(version)?;
    fs::metadata(bundle.join("cache/nix-cache-info"))
        .await
        .wrap_err("runtime bundle has no Nix binary cache")?;
    verify_installer(&bundle).await?;
    Ok(bundle)
}

pub(super) async fn prepare(repository: &Repository, jobs: NonZeroUsize) -> Result<PathBuf> {
    let version = repository.read_runtime_version().await?;
    let destination = repository
        .resolve_data_home()
        .await?
        .join("image")
        .join(&version);
    if fs::try_exists(&destination).await? {
        let bundle = validate(&destination, &version).await?;
        eprintln!("Using offline bundle: {}", bundle.display());
        return Ok(bundle);
    }

    let destination = std::path::absolute(destination)?;
    let parent = destination
        .parent()
        .wrap_err("runtime bundle path has no parent")?;
    fs::create_dir_all(parent).await?;
    // Publish only a finished bundle. Failed preparation leaves no cache entry.
    let temporary = tempfile::Builder::new()
        .prefix(".hulks-")
        .tempdir_in(parent)?;
    let result = temporary.path().join("result");
    // Keep Nix's temporary archives off the potentially quota-limited /tmp tmpfs.
    let nix_tmp = tempfile::tempdir_in("/var/tmp")
        .wrap_err("failed to create Nix temporary directory in /var/tmp")?;
    eprintln!("Preparing package bundle at {}", destination.display());
    let mut build = Command::new("nix");
    build
        .env("TMPDIR", nix_tmp.path())
        .current_dir(&repository.root)
        .args([
            "--extra-experimental-features",
            "nix-command flakes",
            "build",
            "--log-format",
            "bar-with-logs",
            "--print-build-logs",
            ".#packages.aarch64-linux.hulk-runtime",
            "--no-update-lock-file",
            "--max-jobs",
            "1",
            "--cores",
        ])
        .arg(jobs.to_string())
        .arg("--out-link")
        .arg(&result);

    run(&mut build, "building hulk-runtime environment").await?;
    let root = fs::read_link(&result).await?;
    export_cache(&root, temporary.path(), nix_tmp.path()).await?;
    let metadata = fs::read(root.join("share/hulk-runtime/runtime.json"))
        .await
        .wrap_err("failed to read runtime metadata")?;
    let runtime = serde_json::from_slice(&metadata)?;
    let manifest = Manifest { root, runtime };
    fs::write(
        temporary.path().join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )
    .await?;
    download_installer(temporary.path()).await?;
    validate(temporary.path(), &version).await?;
    fs::remove_file(&result).await?;
    fs::rename(temporary.path(), &destination)
        .await
        .wrap_err("failed to publish package bundle")?;
    eprintln!("Prepared offline bundle: {}", destination.display());
    Ok(destination)
}

async fn export_cache(root: &Path, bundle: &Path, nix_tmp: &Path) -> Result<()> {
    let mut copy = Command::new("nix");
    copy.env("TMPDIR", nix_tmp)
        .args([
            "--extra-experimental-features",
            "nix-command",
            "copy",
            "--to",
        ])
        .arg(format!(
            "file://{}?compression=zstd",
            bundle.join("cache").display()
        ))
        .arg(root);
    run(&mut copy, "exporting runtime binary cache").await
}

async fn download_installer(bundle: &Path) -> Result<()> {
    run(
        Command::new("curl")
            .args(["--fail", "--location", "--output"])
            .arg(bundle.join("nix-installer.tar.xz"))
            .arg(NIX_INSTALLER_URL),
        "downloading Nix installer",
    )
    .await
}

async fn verify_installer(bundle: &Path) -> Result<()> {
    let installer = bundle.join("nix-installer.tar.xz");
    let output = Command::new("sha256sum")
        .arg(&installer)
        .output()
        .await
        .wrap_err_with(|| format!("failed to checksum Nix installer {}", installer.display()))?;
    ensure!(
        output.status.success(),
        "failed to checksum Nix installer {} ({}): {}",
        installer.display(),
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    );
    ensure!(
        String::from_utf8(output.stdout)?.split_whitespace().next() == Some(NIX_INSTALLER_SHA256),
        "Nix installer checksum mismatch: {}",
        installer.display()
    );
    Ok(())
}

async fn run(command: &mut Command, operation: &str) -> Result<()> {
    let status = command
        .status()
        .await
        .wrap_err_with(|| format!("failed to start {operation}"))?;
    if !status.success() {
        bail!("{operation} failed: {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_incompatible_or_unsafe_manifests() -> Result<()> {
        let manifest: Manifest = serde_json::from_str(
            r#"{
                "root": "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-hulk-runtime",
                "runtime_version": "2.0.0",
                "system": "aarch64-linux",
                "l4t": "36.4.3"
            }"#,
        )?;
        manifest.validate("2.0.0")?;
        for (field, value) in [
            ("runtime_version", "1.0.3"),
            ("system", "x86_64-linux"),
            ("l4t", "36.4.0"),
            ("root", "/nix/store/../../etc/passwd"),
            (
                "root",
                "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-hulk-runtime;id",
            ),
        ] {
            let mut invalid = serde_json::to_value(&manifest)?;
            invalid[field] = value.into();
            let invalid: Manifest = serde_json::from_value(invalid)?;
            assert!(
                invalid.validate("2.0.0").is_err(),
                "accepted {field}={value}"
            );
        }
        Ok(())
    }
}
