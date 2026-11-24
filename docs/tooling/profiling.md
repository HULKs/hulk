# Profiling on the Robot

Install the current [runtime bundle](pepsi.md#offline-runtime-installation), then upload HULK with `./pepsi upload <robot> --profile with-debug`.
The `with-debug` profile keeps optimizations enabled while retaining the symbols needed to identify functions in the profile.
Keep the exact binary used for the recording.

## Record and inspect

Connect with `./pepsi shell <robot>` and run:

```bash
perf=/nix/var/nix/profiles/hulks/bin/perf
sudo "$perf" record -e cpu-clock -F 99 --call-graph dwarf,16384 -z \
  --pid "$(pidof hulk_ros_z)" -- sleep 30
sudo "$perf" report --stdio
```

This writes `perf.data`. To use hardware counters, check support first with `sudo "$perf" stat -e cycles,instructions -- sleep 1`. DWARF sampling adds overhead; increase the stack size if call chains are truncated.

## Desktop analysis

For Hotspot, copy `perf.data`, the exact HULK binary, and the mapped libraries and debug files.
Preserve absolute paths (including `/nix/store`) under a local sysroot.

```sh
hotspot --sysroot ./robot-sysroot --appPath ./target/robot/aarch64-unknown-linux-gnu/with-debug perf.data
```

A robot sysroot isn't needed when profiling a local binary, such as a simulator.
Use the Flame Graph, Top Down, or Bottom Up views to investigate where time is spent.

The bundle provides matching Nix glibc symbols at `/nix/var/nix/profiles/hulks/lib/debug`; Ubuntu's `libc6-dbg` does not match. Other libraries and kernel analysis require their own matching symbols.
