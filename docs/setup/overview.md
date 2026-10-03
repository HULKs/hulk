# Setup Overview

The current robot stack runs on **Booster K1 robots** with a Jetson computer.
The `hulk_ros_z` executable starts ROS-Z nodes connected through Zenoh. ROS-Z is
our Rust middleware; building HULK does not require the ROS 2 C/C++ runtime.
See the [robotics overview](../robotics/overview.md) for the node architecture.

Follow these steps:

1. Set up the [development environment](development_environment.md), clone the
   repository, and build Pepsi.
2. Configure [team and robot identities](configure_team.md), including each
   robot's Jetson serial number and player number.
3. [Provision the Booster](booster_setup.md) with `./pepsi gammaray`.
4. [Build, upload, and verify HULK](upload.md).

The SDK is an AArch64 cross-compilation container. The robot runs the binary
inside a separate inference-runtime container. These are different images;
installing the SDK on your development machine does not install the robot runtime.

For local development, use the [simulator](../tooling/behavior_simulator.md)
and [Twix](../tooling/twix.md). Continue with the
[contributor workflow](../workflow/getting_started.md) and
[automated checks](../workflow/checks.md).
