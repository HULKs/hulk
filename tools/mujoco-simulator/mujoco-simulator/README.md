# MuJoCo Simulator

This project contains a simulator using mujoco to simulate a K1 robot.
To start the simulator, execute
```bash
uv run main.py
```
Among downloading all dependencies, this will also build the `mujoco_simulator` crate which is implemented in Rust.

When running, the simulator exposes a single websocket at `0.0.0.0:8000`, which handles all communication.
To control a robot in the simulator, execute
```bash
pepsi run mujoco
```
