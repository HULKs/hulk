# Pepsi

Pepsi builds and deploys HULK. Run `./pepsi <subcommand> --help` for options.

## Robot workflow

```sh
./pepsi upload 42
```

`upload` builds HULK, transfers it and its configuration and assets, and restarts the service.

## Interaction with the Robot

Use an IP directly, `42` for `10.1.24.42`, or `42w` for `10.0.24.42`.
Many commands accept multiple robots.

`wifi`, `reboot`, `poweroff`, and `hulk` interact with robots directly, whereas `playernumber` only changes local configuration.
Use `pregame` to assign player numbers, configure Wi-Fi, deploy, and restart HULK.
Use `postgame` to collect logs, shut down HULK, and disconnect Wi-Fi.
To download logs without shutting down HULK, run `./pepsi log download ./logs 42`.

On K1 robots, `gammaray` configures HULK and disables manufacturer controller programs; `boosterize` restores manufacturer control.
See [Remote Control](remote_control.md) for setup, gamepad bindings, and restoration instructions.

## Workstation builds

```sh
./pepsi build -p twix
```

`./pepsi` builds Pepsi itself from the working tree with Cargo on your `PATH`. By default, Pepsi also uses Cargo on the build host's `PATH` for workstation commands. Select it explicitly with `--env path-cargo`. If you need Rust or libraries from the flake, such as ONNX Runtime, run `nix develop` first or use `--env nix-native` to have Pepsi select the workstation Nix shell.

When Pepsi enters a Nix shell for a local or remote build, it stages only the files needed by the shell. Cargo still builds in the checkout.

### Robot SDK

Packages marked `package.metadata.pepsi.cross-compile = true` use the Nix robot SDK. If the flag is missing or `false`, Pepsi uses Cargo on `PATH`. Use `--env nix-robot` to select the robot SDK explicitly. The robot SDK requires Nix on x86_64 Linux, whether the build is local or remote.

```sh
./pepsi sdk prepare                  # optional: warm the cache
./pepsi build crates/hulk_ros_z
```

Without a manifest argument, builds default to the workstation. For robot builds selected with `-p`, pass `--env nix-robot`. Deploy the matching runtime bundle before uploading.

### Publishing the Nix cache

Trusted developers can publish workstation and robot outputs from the locked flake to the private HULKs cache:

```sh
./pepsi cache push
```

This publishes the workstation tools, both Zenoh DDS bridge builds, Jetson ONNX Runtime, the robot runtime, and the `default`, `android`, and `robot-sdk` development environments with their dependencies. The cache includes NVIDIA and Android artifacts. Gammaray offline bundles are separate.

CI publishes on pushes to `main` that change the flake, inputs to these outputs, or the cache workflow. The command reads the token from `ATTIC_TOKEN` in CI and fetches Attic on demand from the locked nixpkgs.

### Build options

Use `--profile` to select a build profile, for example `./pepsi build -p twix --profile release`.
Cargo commands such as `build`, `check`, and `clippy` also accept `--target` to select a target triple.
`upload` and `pregame` let you choose a profile but not a target triple.

### Build directories

Native builds use Cargo's `target` directory by default. Robot builds use `target/robot` and ignore host `CARGO_TARGET_DIR`. Use `--target-dir` to override the output; for remote artifact retrieval it must be relative.

With `--no-build`, use the environment, profile, and target directory that produced the binary. Pepsi does not enter the SDK in this mode.

## Offline runtime installation

Prepare on an internet-connected x86_64 Linux machine with Nix, curl, and `sha256sum`:

```sh
./pepsi gammaray --prepare
```

`--prepare` creates or validates the cached bundle without reading robot configuration or contacting robots.
Bundles are stored in `$XDG_DATA_HOME/hulk/image/{version}`, defaulting to `~/.local/share/hulk/image/{version}`.
`HULK_DATA_HOME` overrides the HULK data directory.
To rebuild the same version, remove its cached bundle directory and run `--prepare` again.
Failed preparation does not publish a partial bundle.

Copy the entire printed bundle directory, then install it without internet access:

```sh
./pepsi gammaray 42 --bundle /path/to/runtime-bundle
```

Only install trusted bundles: signature checks are bypassed for this local import. Without `--bundle`, Gammaray uses the versioned local cache and prepares a missing bundle. Bump `runtime_version` in `hulk.toml` when runtime packages or static configuration change.

Use `--jobs N` to limit compilation parallelism.
Use `./pepsi gammaray --prepare --remote user@buildhost` to build through Nix over SSH and export the bundle directly to the local cache. With no host argument, `--remote` uses the host from `REMOTE_WORKSPACE` or `.REMOTE_WORKSPACE`. Avoid garbage-collecting the remote store during export.

Gammaray installs the runtime and systemd services, not the HULK binary. Use `upload` separately. The runtime requires **L4T 36.4.3**; the robot supplies kernel drivers and Tegra libraries. TensorRT uses the Orin GPU, not DLA. After a runtime upgrade, regenerate the YOLO and VO engine caches on a robot without setting `LD_LIBRARY_PATH` or `LD_PRELOAD`, then verify GPU inference and clean shutdown.

Uploaded binaries built with the robot SDK run directly and use libraries from the installed bundle. Set `HARDWARE_ID` when starting HULK manually.

The active runtime lives at `/nix/var/nix/profiles/hulks`.
The bundle installs the custom Zenoh DDS bridge, which can also be built separately with `nix build .#packages.aarch64-linux.zenoh-bridge-dds`.
`hulks-firewall.service` loads `/etc/hulks.nft` at boot and blocks outbound UDP to destination port 9000.

### Filesystem overlay

Gammaray enables a persistent writable overlay and reboots if needed.
Installing the bundle directly into the lower root is not supported.
Overlay data is stored in `/var/lib/hulks-overlay` on the existing ext4 root and shares its available disk space.
`--clean` clears existing overlay changes.

Static files in `tools/k1-setup/rootfs_overlay/` mirror the robot filesystem. Keep credentials, robot-specific settings, and writable state out of this tree. Gammaray copies these files into the mounted overlay, replacing local edits at those paths without deleting unrelated files. Switching Nix profiles does not roll back the copied files.

#### Boot selection

```sh
./pepsi overlay enable 42
./pepsi overlay disable 42
./pepsi overlay status 42 99
```

`enable` and `disable` select the root for the next reboot, and `status` reports both the active and next-boot roots.
All three actions support multiple robots.
`boosterize` restores manufacturer services and selects the lower root for the next boot.

#### Requirements and system changes

Overlay activation requires an ext4 root, a static `/bin/busybox`, and kernel OverlayFS support.
The lower installation keeps the overlay boot scripts, data, and boot-target configuration, as well as passwordless sudo if Gammaray enabled it there.
The bootloader and initramfs are unchanged.

## Aliveness

`./pepsi aliveness` reports robot status over Ethernet, highlighting issues by default.
Use `--verbose` for full output, `--json` for scripts, or pass robot addresses to filter the results.
The default response timeout is 200 ms; to wait longer, run `./pepsi aliveness --timeout 500 22`.
See [Aliveness](aliveness.md) for protocol details.

## Shell completion

Generate a script for your shell, for example:

```sh
./pepsi completions zsh > _pepsi
```

Dynamic robot suggestions in Zsh, Bash, and Fish require `pepsi` on `PATH`.
With a host Rust toolchain and linker, run `./pepsi install pepsi` outside `nix develop` and add `~/.cargo/bin` to `PATH` if needed.

## Remote compilation

Set up an account on the x86_64 Linux build host and copy your SSH key with `ssh-copy-id user@buildhost`.
Clone the repository on the build host into a dedicated checkout such as `~/hulk`, since synchronization can overwrite or delete remote changes.
In your local checkout, set `REMOTE_WORKSPACE` or create a `.REMOTE_WORKSPACE` file with `user@buildhost:hulk`.

```sh
./pepsi build --remote
```

This syncs your local checkout and builds on the remote host.
`upload` and `pregame` also accept `--remote`.
Use the lab VPN when connecting from outside the lab.
