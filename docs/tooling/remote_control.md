# Remote Control

`./pepsi gammaray <robot>` uploads `tools/k1-setup/child-hulk.ini` to `/opt/booster/Daemon/bin/child.ini`, restarts `booster-daemon`, and disables `joystick_ros2` for HULK remote control.
`./pepsi boosterize <robot>` uploads `tools/k1-setup/child-booster.ini` to the same path, restarts `booster-daemon`, and enables and starts `joystick_ros2`.

## Controls

Put the robot in `Initial`, then press Start on the controller to enable remote control.
Remote control cannot override `Damping`, `Prepare`, or `Stop`.

| Control | Action |
| --- | --- |
| Press Start | Toggle remote control on/off (starts off) |
| Primary state changes | Disable remote control |
| Controller disconnected or input stale | Stand while remote control remains enabled |
| Left stick up/down | Walk forward/backward |
| Left stick left/right | Walk sideways |
| Right stick left/right | Turn the robot |
| D-pad left/right | Move the head left/right |
| D-pad up/down | Move the head up/down |
| Hold left shoulder, L1/LB | Rumpelstilzchen kick |
| Hold right shoulder, R1/RB | Schlong kick |
| Analog triggers, L2/LT and R2/RT | Unbound |
| Right stick up/down | Unbound |

## Restore Booster Controls

Boosterize replaces `/opt/booster/Daemon/bin/child.ini` with the checked-in Booster configuration, which includes the `RemoteController` section, and enables and starts `joystick_ros2`.
