# Kick and get-up requests

## SDK path

Behavior publishes `MotionCommand::VisualKick` or the unit variant
`MotionCommand::StandUp` on `behavior/motion_command`.
`booster_sdk_interface` consumes these commands in the
[Soccer-mode SDK path](overview.md#ros-z-booster-path).

`VisualKick` carries head motion, Ground-frame ball position, kick direction,
and target position, robot orientation relative to Field, and
`KickPower::{Rumpelstilzchen, Schlong}`. The SDK interface maps the selected power
through `booster_interface.kicking.kick_power` (base values `2.3` and `5.0`),
publishes the SDK kick message, and requests visual-kick enablement. Those power
values are SDK inputs, not a documented ball-speed guarantee.

Entering StandUp queues the SDK `get_up` request; continuing StandUp retains
the same request. An injected stand-up command serializes as `"StandUp"`.
See `crates/types/src/motion_command.rs` for the command schema and
`crates/nodes/booster_sdk_interface/src/control.rs` for SDK conversion.
