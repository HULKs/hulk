# Pepsi

Pepsi is our multi-tool for building code, configuring deployments, uploading the robot stack, and interacting with robots.
Run it from the repository root using `./pepsi`.
For detailed arguments, use `./pepsi --help` or `./pepsi <subcommand> --help`.

## Build and Run

Pepsi wraps Cargo commands such as `build`, `run`, `check`, `clippy`, `test`, `nextest`, and `install`.
You can select a package or binary, a build profile, and a native or container execution environment.

For example, to build the ROS-Z robot executable or launch Twix locally:

```bash
./pepsi build crates/hulk_ros_z
./pepsi run twix
```

The positional argument selects a package directory, manifest path, or registered shortcut such as `twix`.
Selecting `crates/hulk_ros_z` reads that package's `cross-compile` metadata and defaults to the Podman K1 SDK environment.
Selecting only `--bin hulk_ros_z` or `--package hulk_ros_z` does not select its manifest for environment detection; without `--env`, that invocation uses native Cargo.

The experimental simulator requires an Alex development-branch checkout; `./simulator` is not available on main. See [Simulator](behavior_simulator.md) for checkout instructions and the additional worktree-only perception/tuning workflows.

## Upload to a Robot

```bash
./pepsi upload <number-or-IP>
```

`upload` builds `hulk_ros_z`, prepares the upload directory with the binary and supporting files, checks the robot's OS version and launcher configuration, uploads the files, and restarts the HULK service.
Use `--no-build` to upload an existing build or `--prepare` to build without uploading.
`--no-restart` leaves HULK stopped: upload still stops the service before transferring files.
Uploads remove remote files absent from the upload directory by default; `--no-clean` preserves those files.
Run `./pepsi upload --help` for the available build and deployment options.

## Robot Interaction and Configuration

Robots are identified by IP address or by number. Number shortcuts resolve as follows:

- `{number}` → `10.1.24.{number}` (Ethernet)
- `{number}w` → `10.0.24.{number}` (Wi-Fi)

Many subcommands can act on multiple robots concurrently.

| Command | Purpose |
| --- | --- |
| `shell` | Open a remote shell or run a command on robots. |
| `ping` | Check robot connectivity. |
| `wifi`, `reboot`, `poweroff` | Manage connectivity and robot power. |
| `hulk` | Control the HULK service. |
| `playernumber`, `location` | Change local parameter configuration. |
| `pregame` | Configure and deploy the playing robots using `deploy.toml`. |
| `log` (alias `logs`), `postgame` | Manage logs and perform post-game cleanup. |
| `gammaray` | Provision a K1 over SSH: configure hostname/networking, dependencies, runtime and HULK services. |
| `boosterize` | Switch robots to Booster services by disabling the HULK services and enabling the Booster services. |
| `tensor-rt-compile`, `hydra-bench` | Compile TensorRT engines and benchmark neural-network latency on a robot. |
| `sdk`, `gamebranch`, `format` | Manage the SDK, create a competition branch, and format repository files. |

### K1 provisioning

```bash
./pepsi gammaray 42 --image-file /path/to/hulk-runtime.tar
```

The robot's Jetson serial number must already be registered in `team.toml`.
`--password` supplies the Booster user's password; `--image-file` optionally loads a Podman runtime image, and `--update-x5-file` optionally installs an X5 updater.
Use `./pepsi gammaray --help` for the current options. This command provisions the existing K1 installation.

On main, `gammaray` also disables the manufacturer `RemoteController` section in `/opt/booster/Daemon/bin/child.ini` and `joystick_ros2` for HULK remote control. `boosterize` restores that section and enables and starts `joystick_ros2` along with the Booster services. See [Remote Control](remote_control.md) for setup, gamepad bindings, and restoration instructions.

## Build Environments and Directories

Use `--env native`, `--env podman`, or `--env docker` to select the build environment.
Container environments use the configured K1 SDK image unless an image override is supplied.
Supply an override as part of the environment value, for example `--env podman:ghcr.io/hulks/k1sdk:<tag>` or `--env docker:ghcr.io/hulks/k1sdk:<tag>`.
If no environment is specified, Pepsi checks the selected manifest's requested environment and otherwise defaults to native.

Native builds, including the `./pepsi` launcher, keep Cargo's default `target` directory.
Podman and Docker builds use `target/container`, mounted at `/hulk/target/container` inside the container.
Separate directories prevent unnecessary rebuilds caused by differing source paths between native and container builds.

Use `--target-dir` to override the directory for a Pepsi build command.
Relative paths are relative to the directory where you invoke Pepsi.
Absolute container paths use the container filesystem; keep them under `/hulk` when you need to retrieve or upload binaries.
Native `CARGO_TARGET_DIR` is not forwarded to containers, so select a custom container directory with `--target-dir`.

`upload`, `pregame`, `tensor-rt-compile`, and `hydra-bench` look for binaries in the selected environment's directory, including with `--no-build`.
Use the same environment, profile, and target directory as the build that produced the binary.
When a command requests artifacts, remote container builds return them to the matching path in your local repository.
For remote native builds that retrieve binaries, use a relative `--target-dir`; absolute paths can refer to different locations on the two machines.

## Shell Completion

Generate shell completions with the `completions` subcommand:

```bash
./pepsi completions zsh > _pepsi
```

Refer to your shell's completion documentation for installation instructions.
Robot-address suggestions depend on a network discovery service, which current
Booster provisioning does not install. Keep `pepsi` in your `PATH` for generated
completion scripts:

```bash
./pepsi install pepsi
```

Add `~/.cargo/bin` to your `PATH` if it is not already present.

## Remote Compilation

Remote compilation requires an account and a dedicated repository checkout or worktree on the remote compiler, plus SSH access from your local machine.
Synchronization uses `rsync --delete`, excludes `.git`, and applies `.gitignore` filters. Remote files absent locally can be deleted and remote edits can be overwritten.
Use a dedicated worktree with no independent work to preserve; the remote checkout is a build workspace for your local tree.
Configure passwordless SSH and create a `.REMOTE_WORKSPACE` file in your local repository containing the remote account and checkout path, for example:

```text
<name>@remote-compiler.hulks.dev:hulk-remote-build
```

Then run:

```bash
./pepsi build crates/hulk_ros_z --remote
```

Pepsi syncs local files and runs the command remotely. A plain `build --remote` does not request artifact retrieval; commands such as `upload` request the binary they need.
For example, `./pepsi upload 42 --remote` builds remotely, retrieves the robot binary, and uploads it.
Commands such as `run`, `upload`, and `pregame` also support `--remote`.
A team VPN connection is required to access the remote compiler from outside the lab.
