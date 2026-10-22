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

Boosterize uploads `tools/k1-setup/containerd-booster.yaml` to `/opt/booster/config/containerd/config.yaml`.
This configuration enables these Booster services:

- `remote_controller`
- `ros.booster.vision_detection`
- `ros.booster.rpc_bridge.perception`
- `ros.agent_manager`
- `lui`
- `rtc_client`

Gammaray uploads `tools/k1-setup/containerd-hulk.yaml`, which disables the same services.
Both commands stop HULK before uploading the template and restart `booster-daemon` on every run to apply it.
Boosterize also enables and starts `joystick_ros2`.
