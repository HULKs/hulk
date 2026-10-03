# Hardware Integration

`crates/hulk_ros_z/src/main.rs` starts the ROS-Z hardware interface and bridge
nodes alongside the behavior, perception, and motion nodes. Booster communication
uses the host Zenoh router and DDS bridge configured by gammaray.
See the [robotics overview](../robotics/overview.md) and
[Booster motion path](../robotics/motion/overview.md#ros-z-booster-path).

The robot's application lifecycle is managed by `hulk.service` and
`hulk-runtime.service`; use [Booster setup](../setup/booster_setup.md) and
[Home Directory](home_directory.md) for provisioning and diagnostics.
