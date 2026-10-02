# Uploading HULK

First [provision the Booster K1](booster_setup.md) and configure its
[identity, parameter layers, and player number](configure_team.md).
Run from the repository root:

```sh
./pepsi upload <robot-number-or-IP>
```

`upload` selects `crates/hulk_ros_z/Cargo.toml`, builds the AArch64
`hulk_ros_z` executable in the SDK container by default, and prepares the binary,
`etc` assets/parameters, and logs/source directory for deployment.
See [SDK management](development_environment.md#build-pepsi-and-install-the-sdk)
for explicit image installation and missing-image build behavior.

For each robot, Pepsi checks connectivity, compares the vendor OS version with
`hulk.toml`, and checks that `/usr/bin/launch-hulk` launches `hulk_ros_z`.
It stops HULK, uploads into the `booster` user's `hulk` directory with rsync,
and starts HULK again. The launcher check reports that gammaray is needed if the
installed launcher does not name the expected executable.

## Options and Existing Builds

```sh
./pepsi upload --help
```

- `--no-build`: upload an existing binary. Use the same environment, profile,
  and target directory as the build that produced it.
- `--no-restart`: leave HULK stopped after uploading; upload still stops it first.
- `--no-clean`: retain remote files that are not in the upload. By default rsync
  deletes remaining/excluded remote files, so download logs you want to keep first.
- `--skip-os-check`: bypass the vendor OS version comparison.
- `--prepare`: build/resolve the binary without uploading; it does not retain
  a prepared deployment directory.

Build options such as `--release`, `--env`, `--target-dir`, and `--remote` are
available. Default SDK artifacts are under
`target/container/aarch64-unknown-linux-gnu/<profile>/`; see
[Pepsi build directories](../tooling/pepsi.md#build-environments-and-directories).

## Check Startup and Logs

A completed transfer is not proof that every ROS-Z node and inference model is
working. Check the robot's services and logs:

```sh
./pepsi hulk status <robot-IP>
./pepsi log show <robot-IP>
./pepsi shell <robot-IP> "sudo journalctl -u hulk -u hulk-runtime --no-pager -n 100"
```

The executable is `/home/booster/hulk/bin/hulk_ros_z`. Application output is in
`/home/booster/hulk/logs/latest/hulk.out` and `hulk.err`; see
[Home Directory](../operating_system/home_directory.md) for the complete layout.
Verify model inference and GPU provider selection on the actual robot when
changing inference dependencies. Use [Twix](../tooling/twix.md) with the robot's
namespace (for example `/42`) and a reachable Zenoh router endpoint.

For game deployments, `./pepsi pregame` configures and deploys the playing robots
from `deploy.toml`. Continue with the [workflow](../workflow/overview.md).
