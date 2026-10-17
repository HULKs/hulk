# Pepsi

Pepsi is a multi-tool we use for anything related to the code or the robots.
It can be used to build the code, set up configuration parameters for a game, deploy to a robot or simply open a remote shell.

This page is only meant as a general overview of pepsi's subcommands.
For detailed usage instructions, run `pepsi --help` or `pepsi <subcommand> --help`.

## Typical Robot Workflow

```bash
./pepsi upload <number or IP>
```

This command does the following:

- checks if a toolchain is installed, downloads, and installs one if necessary
- builds the code for the Booster target
- uploads binary, configuration parameters, motion files, neural networks, etc. to the robot(s)
- restarts HULK service on the robot(s)

## Interaction with the Robot

Robots are identified either by IP or by number.
Numbers are converted to IPs as follows:

- `{number}` -> `10.1.24.{number}`
- `{number}w` -> `10.0.24.{number}`

Many subcommands can act on multiple robots concurrently.

`upload` builds a binary for the Booster target, and then uploads it and parameter files to one or more robot.

`wifi`, `reboot`, `poweroff`, and `hulk` directly interact with the robot(s), whereas `communication`, and `playernumber` only change the local configuration parameters.

`pregame` combines deactivating communication (to avoid sending illegal messages), assigning playernumbers, setting a wifi network, uploading, and restarting the HULK service.

`logs` or and `postgame` can be used after a (test-)game to download logs, the latter also shuts down the HULKs binary and disables wifi.

On K1 robots, `gammaray` configures HULK and disables manufacturer controller programs; `boosterize` restores manufacturer control.
See [Remote Control](remote_control.md) for setup, gamepad bindings, and restoration instructions.

### Offline gammaray preparation

Prepare packages on an internet-connected Linux host with Nix and curl.
The existing flake lock pins Podman, Zenoh, nftables, and their dependencies.
ARM64 binaries are fetched from the Nix cache; cache misses require an ARM64 builder or emulation.
Environment assembly runs on x86_64 Linux.

```sh
pepsi gammaray --prepare
```

`--prepare` creates the bundle if absent, then exits before reading robot configuration or contacting robots.
An existing bundle is reused without Nix commands or downloads.
Gammaray also prepares a missing bundle, so run `--prepare` while online before gammaraying offline.
Bundles are stored in `$XDG_DATA_HOME/hulk/image/{version}`, defaulting to `~/.local/share/hulk/image/{version}`.
`HULK_DATA_HOME` overrides the HULK data directory.
`version` is `runtime_version` from `hulk.toml`.
Bump that version after runtime package changes, or remove the directory to rebuild.
Failed preparation does not publish a partial bundle.

The bundle includes a version-pinned ARM64 Nix installer, complete Nix runtime closure, and root store path.
The robot needs SSH, rsync, sudo, Bash, tar/xz, systemd, NVIDIA tooling, and kernel support for nftables.

### Filesystem overlay

Gammaray runs inside a persistent writable OverlayFS root.
All changes to the filesystem while the overlay is active are written to the overlay instead of the lower root filesystem.
Changes in the overlay survive reboot while the underlying NVIDIA L4T and Booster installation stays unchanged.
Overlay data is stored in `/var/lib/hulks-overlay` on the existing ext4 filesystem and shares available disk space.

#### Gammaraying a robot

```sh
./pepsi gammaray 42
```

Gammaray first enables overlay autostart and reboots if the overlay is not already active.
If there is an overlay already, it is used instead, though `--clean` can be used to clear the overlay and start fresh.

Afterwards, gammaray writes our HULKs OS configuration into the overlay.
It is not currently possible to gammaray into the lower root filesystem without an overlay.

#### Boot selection

```sh
./pepsi overlay enable 42
./pepsi overlay disable 42
./pepsi overlay status 42 99
```

`enable` and `disable` select whether the overlay or the underlying installation should be used on boot.
They work from either root, but do not apply immediately.
A reboot is required to change into/out of the overlay.
`status` reports both whether the overlay is currently active and whether it will be used on boot.
All three actions support multiple robots.

`pepsi boosterize 42` disables overlay autostart and restores Booster services in the currently running system.
The next reboot uses the underlying installation.

#### Requirements and system changes

Our overlay activation process supports NVIDIA L4T with systemd 249, an ext4 root, a static `/bin/busybox`, and kernel OverlayFS support.
It works both on gammarayed and stock robots.

Gammaray enables passwordless sudo before activation; when performed on the underlying installation, this change persists there.
Overlay boot scripts, data, and boot-target configuration also reside on the underlying installation.
An early systemd target activates the overlay after device initialization.
Bootloader and initramfs configuration are unchanged.

### Offline runtime installation

Build/install Pepsi and prepare the runtime-image tar before disconnecting:

```sh
pepsi gammaray 42 --image-file /path/to/hulk-runtime.tar
```

With the bundle already prepared, neither host nor robot needs internet during this command.
Nix is bootstrapped from the local installer without channels; the closure is imported without building or fetching packages.
The active runtime lives at `/nix/var/nix/profiles/hulks`, retained by a Nix profile.
Ubuntu systemd runs the supplied Zenoh services and Nix Podman's Quadlet generator.
NVIDIA drivers and CDI tooling remain part of the robot baseline.

`hulks-firewall.service` blocks outbound UDP destination port 9000 for IPv4 and IPv6.
It loads `/etc/hulks.nft` at boot and atomically replaces only the `inet hulks` table on reload.
The runtime requires this service.
Gammaray removes its old UFW UDP/9000 rule after activating nftables; other firewall rules and policies remain in effect.
No Debian packages or UFW installation are needed.

The image tar must contain `hulk-runtime:latest`.
It may be omitted when that image already exists in rootful Podman storage; registry pulls are disabled.
Services are stopped during activation.
Package/profile rollback alone does not undo Podman database migrations.
Network reconfiguration runs last, and robot failures are reported individually.

## Build Options

For subcommands that build a binary, you can specify a target and a build profile.
These include `build`, `run`, `check`, and `clippy`.
However `upload` and `pregame` only supports a profiles, since it doesn't make sense to upload a webots binary to the booster.

### Build directories

Native builds, including the `./pepsi` launcher, keep Cargo's default `target` directory.
Podman and Docker builds use `target/container`, mounted at `/hulk/target/container` inside the container.
Separating these directories prevents Cargo from repeatedly rebuilding dependencies whose source paths differ between native and container builds.
The first container build after this change needs to populate the new cache; existing artifacts are left in place.

Use `--target-dir` to override the directory for a Pepsi build command.
Relative paths are relative to the directory where you invoke Pepsi.
Absolute container paths use the container filesystem; keep them under `/hulk` when you need to retrieve or upload binaries.
Native `CARGO_TARGET_DIR` is not forwarded to containers, so a custom container directory must be selected with `--target-dir`.
Choosing the same directory for both environments can bring back the invalidation.

`upload`, `pregame`, `tensorrt-compile`, and `hydra-bench` look for binaries in the selected environment's directory, including with `--no-build`.
Use the same environment, profile, and target directory as the build that produced the binary.
Remote container builds return binaries from the selected container directory to the matching path in your local repository.
For remote native builds that retrieve binaries, use a relative `--target-dir`; absolute paths can refer to different locations on the two machines.

## Aliveness

Using the `aliveness` subcommand, pepsi can query information from robots connected via ethernet.
By default, only irregular information like non-active services, outdated HULKs-OS versions and battery charge levels below 95% are displayed.
Using `-v`/`--verbose` or `-j`/`--json`, you can retrieve all information available via aliveness in either a human- or machine-readable format.

You can also set a timeout via `-t`/`--timeout` (defaulting to 200ms) and specify robot addresses (e.g. `22` or `10.1.24.22`) for querying the aliveness information only from specific robots.

Further information on the information available via aliveness as well as the details to the protocol can be found [here](./aliveness.md).

## Shell Completion

Shell completions can be generated using the `completions` subcommand.

Example:

```bash
./pepsi completions zsh > _pepsi
```

Refer to your shell's completion documentation for details.

The shells completions for fish, zsh and bash include dynamic suggestions for all pepsi subcommands taking a robot address as an argument (e.g. `pepsi upload`).
Those suggestions are retrieved using the aliveness service and require a version of pepsi to be installed in the `PATH`, e.g. by using

```
./pepsi install pepsi
```

and adding `~/.cargo/bin` to the `PATH`.

## Remote Compile

To use the remote compilation you need to create an account on the remote-compiler.
Open an ssh connection to ```root@remote-compiler.hulks.dev```.
There create a new account by ```adduser {name}``` and set a password with ```passwd {name}```.

Terminate the root ssh session and log in with your new user ```{name}@remote-compiler.hulks.dev```.
There clone the HULKs repository using https: ```https://github.com/HULKs/hulk.git```

Back on your local machine do ```ssh-copy-id {name}@remote-compiler.hulks.dev``` to allow passwordless login.
In the hulk repo, create a `.REMOTE_WORKSPACE` file containing the username, IP, and path, e.g. `{name}@remote-compiler.hulks.dev:hulk`.

Now you can use the pepsi remote features:

```bash
./pepsi build --remote
```

This will sync your local files to the remote, run the build command there, and then return the final binary to you.
Other pepsi commands such as `run`, `upload`, or `pregame` also have a `--remote` option.

To use the remote compile functionality from outside the lab, you need a VPN connection.
Ask one of the older team members to provide you a `.ovpn` file.
Create a new VPN client with this configuration file.
Using the VPN, you can access the remote compiler and all other internal services from outside the lab.
