# MuJoCo Simulator

This is the Python MuJoCo K1 simulator and its WebSocket server available on main.
An experimental in-process ROS-Z behavior/motion simulator lives on
[Alex's `motion-inference-simulator` branch](https://github.com/alexschmander/hulk/tree/motion-inference-simulator).
That alternative requires a development-branch checkout with its `simulator`
launcher and `tools/simulate` implementation; neither is available on main.

## Start the Python server

Run from `tools/mujoco-simulator/mujoco-simulator` with Python 3.13 and `uv`:

```bash
uv run main.py
```
Dependency installation also builds the `mujoco-rust-server` Rust extension,
imported by Python as `mujoco_rust_server`.

The default WebSocket bind address is `0.0.0.0:8000`. Override it with:

```bash
uv run main.py --bind-address 127.0.0.1:8000
```

## Client status

The former `pepsi run mujoco` recipe is not available in the current workspace:
Pepsi has no `mujoco` manifest shortcut or corresponding runnable binary.
Starting this server alone does not connect the current robotics stack.
A compatible WebSocket client must send the `ConnectionInfo` handshake and the
binary messages defined in `crates/simulation_message`; the server implementation
is in `tools/mujoco-simulator/mujoco-rust-server/src/websocket.rs`.
There is currently no end-to-end client setup recipe for this older tool.
