# MuJoCo Simulator

## Interactive K1 viewer

From the repository root, run `./pepsi mujoco-viewer` to open the standalone viewer.
From this directory, you can also run `uv run viewer.py` directly.
Press **F2** (or enable **Option → Info**) to show FPS.

On WSL, the script automatically launches Windows Python with native NVIDIA/AMD/
Intel OpenGL instead of Mesa/D3D12. Use the same command:

```sh
uv run viewer.py
```

This requires Windows `uv.exe` on your WSL `PATH`. It starts Windows Python
with native OpenGL, reading the same `viewer.py` and K1 model through WSL's
shared filesystem. Dependencies are managed in a separate Windows uv cache;
the Linux environment and lockfile are unchanged. The first launch downloads
MuJoCo 3.3.6 and its Python dependencies. Shadows and reflections stay enabled.

This bypasses the [slow WSL rendering reported upstream](https://github.com/google-deepmind/mujoco/issues/1008).
On native Linux, Windows, and macOS, the script uses the current Python runtime.

This opens a standalone simulation, not a client of the WebSocket server below.

## Simulation server

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
