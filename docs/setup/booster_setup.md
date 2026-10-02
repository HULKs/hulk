# Booster K1 Setup

This guide assumes a working [development environment](development_environment.md)
and an already installed Booster OS. The expected vendor OS version is
`os_version` in root `hulk.toml`; `upload` compares it with the last `Version:`
entry in `/opt/booster/version.txt`. Obtain vendor OS installation/update
instructions and runtime-image artifacts from the team or vendor for the
specific robot. `gammaray` provisions an existing OS installation.

## Initial Access and Identity

Connect to the robot's existing reachable IP address. The current Pepsi robot
connection helper uses SSH user `booster` and the built-in password `123456`.
The `gammaray --password` option supplies the password for enabling sudo; it
does not override the SSH helper's password. Initial access therefore needs to
work with the helper's credentials or existing SSH authentication.

```sh
./pepsi shell <initial-IP>
```

The robot must already have SSH, rsync, `jetson_release`, NetworkManager, APT,
and the NVIDIA container toolkit (`nvidia-ctk`) available. Register its serial
number in [team configuration](configure_team.md) before provisioning.

## Provision with Gammaray

```sh
./pepsi gammaray --help
./pepsi gammaray <initial-IP>
```

The command accepts one or more robot numbers or full IP addresses. Use the
initial full IP until the configured network addresses are available.
Its options are:

| Option | Purpose |
| --- | --- |
| `-p`, `--password <PASSWORD>` | Password for enabling passwordless sudo; default `123456`. |
| `-i`, `--image-file <PATH>` | Upload and load a runtime Podman image archive. |
| `-u`, `--update-x5-file <PATH>` | Upload an `update_x5` executable and run it with sudo. |

For a first runtime installation, supply an image archive unless
`hulk-runtime:latest` is already installed in the robot's rootful Podman image
store. The archive must provide the image tag used by
`tools/k1-setup/hulk-runtime.container`; it is separate from `k1sdk`.

```sh
./pepsi gammaray --image-file <runtime-image.tar> <initial-IP>
```

### What Provisioning Changes

The implementation in `tools/pepsi/src/gammaray.rs`:

- Matches the Jetson serial to `team.toml`, enables passwordless sudo for
  `booster`, and sets the hostname.
- Modifies NetworkManager's `Wired connection 2`, setting the team Ethernet
  address while retaining `192.168.10.102/24` for robot services. It sets the
  wired gateway and cloned MAC address, then brings the connection up.
- Writes Wi-Fi profiles for `HSL_A`–`HSL_J` and `HSL_HULKs`, then restarts
  NetworkManager. See [network addresses](configure_team.md#network-addresses).
- Adds the Zenoh APT source and installs `zenohd`, `zenoh-bridge-dds`, and `ufw`
  if needed. It enables UFW with a default-allow policy and an outgoing UDP
  port-9000 deny rule.
- Generates NVIDIA CDI configuration and installs/verifies Podman using the
  bundled installation script. The script targets Podman `5.8.1`; when another
  version is installed, it stops all Podman containers, removes the APT Podman
  and crun packages, and installs the static release under `/usr/local`.
- Uploads the Zenoh router and DDS bridge configurations, service overrides,
  HULK service, clock-refresh timer, launch scripts, and runtime Quadlet.
- Creates the TensorRT cache directory, optionally loads the image archive,
  and optionally runs `update_x5`.
- Reloads systemd, enables/restarts Zenoh services, enables the clock-refresh
  timer, recreates `hulk-runtime.service`, and enables/starts `hulk`.
- Disables the vendor `booster-daemon-perception`, `booster-agent-manager`,
  `booster-lui`, and `booster-rtc-speech` services.

Network configuration may change the address through which you connected.
Reconnect using the configured Ethernet address after provisioning. Gammaray
also attempts to start HULK before you have uploaded the binary; check its
per-robot output, then [upload HULK](upload.md).

## Verify the Runtime

```sh
./pepsi shell <robot-IP> "sudo systemctl status hulk-runtime hulk zenohd zenoh-bridge-dds"
./pepsi shell <robot-IP> "sudo podman images"
./pepsi shell <robot-IP> "sudo journalctl -u hulk-runtime -u hulk --no-pager -n 100"
```

After upload, verify model loading, inference, and startup/shutdown on the robot.
See [inference compatibility](development_environment.md#inference-runtime-compatibility)
and [runtime layout](../operating_system/home_directory.md).
