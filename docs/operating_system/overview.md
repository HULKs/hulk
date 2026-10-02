# Booster Operating Environment

The current HULK stack runs on the Booster K1's Jetson computer with the vendor
Linux installation and a separate Podman inference-runtime container.
`os_version` in root `hulk.toml` names the expected vendor release; Pepsi reads
the robot's version from `/opt/booster/version.txt` before upload.

There are two distinct container environments:

- **Development SDK:** `k1sdk`, used on the development/build host to
  cross-compile for AArch64. See [development setup](../setup/development_environment.md).
- **Robot runtime:** `hulk-runtime:latest`, used on the robot to execute
  `/home/booster/hulk/bin/hulk_ros_z` with ONNX Runtime and NVIDIA libraries/devices.

`tools/k1-setup/` contains the provisioning configuration and scripts.
[Gammaray](../setup/booster_setup.md) installs them; it does not flash the vendor OS.
Systemd manages `hulk-runtime.service` (generated from the Quadlet) and
`hulk.service`. HULK requires the runtime service and starts after it.
The runtime has host networking, NVIDIA CDI GPU access, sound-device access,
and a bind mount of `/home/booster`.

The host also runs `zenohd` and `zenoh-bridge-dds`. The robot launcher connects
HULK to `tcp/127.0.0.1:7447`; ROS-Z namespaces are derived from robot numbers.
See [team configuration](../setup/configure_team.md) and
[hardware integration](hula.md).

For operations, see [Home Directory](home_directory.md), [WiFi](wifi.md),
[Linux](linux.md), and [storage guidance](partitioning.md).
The previous Yocto distribution is documented in
[Historical: NAO Operating System](../historical/operating_system/overview.md).
