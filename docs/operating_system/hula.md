# Hardware Integration and Legacy HULA

The current Booster ROS-Z stack does not use HULA or NAO's LoLA service.
`crates/hulk_ros_z/src/main.rs` starts the ROS-Z hardware interface and bridge
nodes alongside the behavior, perception, and motion nodes. Booster communication
uses the host Zenoh router and DDS bridge configured by gammaray.
See the [robotics overview](../robotics/overview.md) and
[Booster motion path](../robotics/motion/overview.md#ros-z-booster-path).

The robot's application lifecycle is managed by `hulk.service` and
`hulk-runtime.service`; use [Booster setup](../setup/booster_setup.md) and
[Home Directory](home_directory.md) for provisioning and diagnostics.

## Legacy NAO HULA

HULA is the older NAO abstraction layer that connects applications to LoLA and
supports multiple clients. Its design diagrams and old build instructions are
preserved in [Historical: HULA](../historical/operating_system/hula.md).

There is no `tools/hula/Cargo.toml` in the current source tree. Current
`./pepsi sdk install` pulls a K1 container image; it does not install the local
NAO SDK expected by those instructions. To rebuild HULA, use the legacy source
revision and compatible NAO image/SDK selected by the external `meta-nao` recipe.
