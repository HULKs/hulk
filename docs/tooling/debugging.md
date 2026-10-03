# Debugging with GDB/LLDB

## Build with symbols

Run commands from the repository root. `with-debug` retains the normal development
optimizations and adds debug symbols; `debugger` also disables optimizations for
stepping through code.

For example, build and run Twix under GDB:

```sh
./pepsi build twix --env native --profile debugger
gdb --args target/debugger/twix /42 --router tcp/10.1.24.42:7447
```

Use a reachable router and the namespace of your robot. In GDB, use `run`,
`thread apply all bt` to inspect thread stacks, and `break <function>` to set a
breakpoint. With LLDB:

```sh
lldb -- target/debugger/twix /42 --router tcp/10.1.24.42:7447
```

Use `run` and `thread backtrace all`. Replace `target` with your configured
Cargo target directory when overridden.

For another local Rust binary, build its manifest with
`./pepsi build <manifest-directory> --env native --profile debugger` and launch
the executable under the debugger. The [behavior tree simulator](behavior_tree_simulator_design.md)
provides local scenario executables for debugging behavior decisions.

For robot-target symbols, use `./pepsi build crates/hulk_ros_z --profile with-debug`
or `./pepsi upload 42 --profile with-debug`. These select the K1 SDK environment
and produce an AArch64 binary. Debugging that binary requires a debugger for the
robot's architecture and access to its runtime libraries/container.

## Inspect ROS-Z before attaching a debugger

Install the CLI:

```sh
./pepsi install ros-z-cli
```

Inspect the running robot's graph and bounded samples:

```sh
rosz --router tcp/10.1.24.42:7447 list nodes
rosz --router tcp/10.1.24.42:7447 list topics
rosz --router tcp/10.1.24.42:7447 info topic /42/behavior/motion_command
rosz --router tcp/10.1.24.42:7447 echo /42/behavior/motion_command --count 1 --timeout 5
rosz --router tcp/10.1.24.42:7447 hz /42/behavior/motion_command --duration 5
rosz --router tcp/10.1.24.42:7447 parameter snapshot --node /42/behavior_node
```

Replace the endpoint and namespace with those of your robot.
[Twix](twix.md) provides topic and parameter inspection interactively.

Set `RUST_BACKTRACE=1` when reproducing a panic. A debugger can pause the process
it attaches to, so debugging the robot stack can interrupt control. A paused
process is not a timing or performance measurement; see [Profiling](profiling.md)
for optimized sampling.
