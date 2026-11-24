use std::path::Path;

use tokio::process::Command;

pub fn robot_command(root: &Path) -> Command {
    shell_command(root, "robot-sdk")
}

pub fn native_cargo_command(root: &Path) -> Command {
    let mut command = native_command(root);
    command.arg("cargo");
    command
}

pub fn native_command(root: &Path) -> Command {
    shell_command(root, "default")
}

pub fn push_cache_command(root: &Path) -> Command {
    shell_command(root, "push-cache")
}

fn shell_command(root: &Path, shell: &str) -> Command {
    let mut command = Command::new("bash");
    command
        .args([
            "-c",
            include_str!("../../../tools/run-nix-shell"),
            "hulk-sdk",
        ])
        .arg(root)
        .arg(shell);
    command
}
