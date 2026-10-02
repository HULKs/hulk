# Debugging with GDB/LLDB

## Build with symbols

Run commands from the repository root. `with-debug` retains the normal development optimizations and adds debug symbols; `debugger` additionally sets optimization level zero for easier stepping.

### Experimental simulator (development checkout required)

The simulator examples below require the [Alex development-branch checkout](behavior_simulator.md#alexs-development-branch); main has no `simulator` launcher or `simulate` crate. Source paths in these examples refer to that branch, not main.

For an interactive local simulator in that checkout:

```bash
./simulator --no-robotics --help
```

This exercises the launcher's MuJoCo setup and build without loading motion models or opening a window.
Before building directly with Cargo, configure the MuJoCo library discovery used by `tools/simulate` (Bash syntax):

```bash
export MUJOCO_NO_PKG_CONFIG=1
export MUJOCO_DOWNLOAD_DIR="${MUJOCO_DOWNLOAD_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/mujoco-rs}"
export LD_LIBRARY_PATH="$MUJOCO_DOWNLOAD_DIR/mujoco-3.9.0/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
cargo build -p simulate --profile debugger
gdb --args target/debugger/simulate --no-robotics
```

Use your Cargo target-directory override in place of `target` if configured. In GDB, use `run`, `thread apply all bt` to inspect thread stacks, and `break <function>` to set a breakpoint. With LLDB:

```bash
lldb -- target/debugger/simulate --no-robotics
```

Use `run` and `thread backtrace all`. Full robotics mode additionally needs the K1 model files and `ORT_DYLIB_PATH`; see [Simulator](behavior_simulator.md).

For another local Rust binary, build its manifest with `./pepsi build <manifest-directory> --env native --profile debugger` and launch the resulting executable under the debugger.
For robot-target symbols, use `./pepsi build crates/hulk_ros_z --profile with-debug` or `./pepsi upload 42 --profile with-debug`. These select the K1 SDK environment and produce an aarch64 binary; a host debugger cannot run that binary as a native host process. Debugging on the robot requires a matching debugger and access to its runtime libraries/container.

## Inspect ROS-Z before attaching a debugger

Install the CLI once:

```bash
./pepsi install ros-z-cli
```

With the experimental simulator running in its development checkout, inspect its graph and bounded samples:

```bash
rosz --router tcp/127.0.0.1:7447 list nodes
rosz --router tcp/127.0.0.1:7447 list topics
rosz --router tcp/127.0.0.1:7447 info topic /simulator/robot/behavior/motion_command
rosz --router tcp/127.0.0.1:7447 echo /simulator/robot/behavior/motion_command --count 1 --timeout 5
rosz --router tcp/127.0.0.1:7447 hz /simulator/robot/behavior/motion_command --duration 5
rosz --router tcp/127.0.0.1:7447 parameter snapshot --node /simulator/robot/behavior_node
```

Physics and recurring behavior outputs stop while paused. Parameter services remain available; resume before measuring behavior frequency.
For a robot, replace the router and namespace with that robot's reachable endpoint and namespace, such as `/42`. Twix provides the same topic and parameter inspection interactively; see [Twix](twix.md).

Set `RUST_BACKTRACE=1` when reproducing a panic. A debugger stops the process it attaches to, including the in-process simulator robotics tasks, so a paused debugger is not a timing or performance measurement. See [Profiling](profiling.md) for optimized sampling.
