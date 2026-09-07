# Remote Control

`./pepsi gammaray <robot>` disables the manufacturer controller services for HULK remote control.
`./pepsi boosterize <robot>` restores the `RemoteController` section in `/opt/booster/Daemon/bin/child.ini` and enables and starts `joystick_ros2`.

## Controls

| Control | Action |
| --- | --- |
| Left stick up/down | Walk forward/backward |
| Left stick left/right | Walk sideways |
| Right stick left/right | Turn the robot |
| D-pad left/right | Move the head left/right |
| D-pad up/down | Move the head up/down |
| Hold left shoulder, L1/LB | Rumpelstilzchen kick |
| Hold right shoulder, R1/RB | Schlong kick |
| Analog triggers, L2/LT and R2/RT | Unbound |
| Right stick up/down | Unbound |

## Restore Manufacturer Controls

Boosterize restores the `RemoteController` section in `/opt/booster/Daemon/bin/child.ini` and enables and starts `joystick_ros2`.
