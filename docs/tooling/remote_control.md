# Remote Control

`./pepsi gammaray <robot>` disables the Booster controller services for HULK remote control.
`./pepsi boosterize <robot>` restores the Booster services and enables and starts `joystick_ros2`.

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

Gammaray patches Booster's startup script to keep an existing runtime configuration.
Booster's platform defaults are initialized only when `/etc/booster/config/containerd/config.yaml` is missing or empty.
The patch is applied in the overlay and rejects unrecognized startup scripts.

Gammaray uses `bdb` to disable:

- `remote_controller` and `ros.agent_manager`
- all services in the `perception` container except `battery_daemon`, including `audio`
- all X5 services, including the left and right hand ROS bridges in the `motion` container

It also removes the `battery_daemon` dependency on `audio` and the `server.motion` dependency on `server.perception`.
The draft is validated and applied through `bdb`, then `booster-daemon` is restarted to load the saved configuration into the live service list.
The settings survive daemon restarts and robot reboots.
Older template overrides in the overlay are replaced with the vendor template from the underlying installation.
No complete Booster configuration is stored in the repository or transferred from the host.

Boosterize deliberately regenerates the runtime configuration from Booster's template and platform defaults to restore manufacturer services and dependencies.
It also enables and starts `joystick_ros2`; gammaray disables and stops it.
After a firmware update replaces the startup script, run gammaray again to reapply the patch.
