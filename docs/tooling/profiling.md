# Profiling with perf

Build the binary with debug symbols before recording. The `with-debug` profile inherits the optimized development profile and enables symbols; `debugger` disables optimizations and is intended for stepping rather than representative profiling.

## Local tools

For example, from the repository root:

```bash
./pepsi build twix --profile with-debug
perf record --call-graph dwarf -o twix.perf.data -- target/with-debug/twix /42 --router tcp/10.1.24.42:7447
```

Interact with Twix while recording, then close it to finish the profile. This requires `perf` on the host and kernel permissions for performance counters.
Use a reachable router and namespace for your running robot stack.
Use your configured Cargo target directory instead of `target` when overridden.

## K1 robot stack

Build and upload the symbol-bearing robot executable:

```bash
./pepsi upload 42 --profile with-debug
./pepsi shell 42
```

The default SDK build artifact is `target/container/aarch64-unknown-linux-gnu/with-debug/hulk_ros_z`.
Keep the exact binary used for the recording; rebuilding later can make symbols and addresses disagree.

The provisioned K1 launcher runs `hulk_ros_z` in the `hulk` Podman container, with the host home directory mounted inside it.
On the robot, check that a host `perf` compatible with its running kernel is installed (`perf --version`); its presence is not guaranteed by the repository's provisioning package list.
Find the host-visible PID and record a bounded sample:

```bash
pidof hulk_ros_z
sudo perf record --call-graph dwarf -o /home/booster/hulk/logs/hulk.perf.data \
  --pid "$(pidof hulk_ros_z)" -- sleep 30
```

This samples the running process and its threads for 30 seconds. Select one PID explicitly if several instances are running.
Using host `perf` avoids depending on a profiler inside the runtime container. Permissions, kernel support and access to container processes must be checked on the target robot.

Back on the host, retrieve the recording:

```bash
rsync -av booster@10.1.24.42:/home/booster/hulk/logs/hulk.perf.data .
```

## Inspect the recording

For the K1 profile, launch Hotspot with the matching binary directory:

```bash
hotspot --appPath ./target/container/aarch64-unknown-linux-gnu/with-debug/
```

Open `hulk.perf.data`, then use the Flame Graph, Top Down or Bottom Up views.
For the local Twix example, use `--appPath ./target/with-debug/` and open `twix.perf.data` instead.

To resolve shared-library frames from a robot, supply `--sysroot <directory>` containing the matching runtime/container libraries and debug symbols with their recorded paths. A native host installation is not a substitute for the K1 aarch64 runtime, and libraries without debug symbols may remain unresolved.
For kernel frames, obtain symbols from that robot/kernel if needed.
