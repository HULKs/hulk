# Bevyhavior Simulator (Historical)

!!! note "Historical documentation"

    This page preserves the original Bevyhavior Simulator instructions. See [Simulator](../tooling/behavior_simulator.md) for the current development status.

A simplified simulator which can be used for manual or automatic testing of behavior in a defined scenario.

# Usage

```sh
./pepsi run --bin golden_goal
```

After the simulation is finished, the simulator opens a commmunication server.
It returns an error if the robotics code encountered a problem or if the scenario file generated an error.

# Scenario Development

Scenario files can be found at `crates/bevyhavior_simulator/src/bin/`.
