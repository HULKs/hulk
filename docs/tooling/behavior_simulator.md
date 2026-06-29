# Bevyhavior Simulator

A simplified simulator which can be used for manual or automatic testing of behavior in a defined scenario.

# Usage

```sh
./pepsi run --bin golden_goal
```

After the simulation is finished, the simulator opens a commmunication server.
It returns an error if the robotics code encountered a problem or if the scenario file generated an error.

# Scenario Development

Scenario files can be found at `crates/bevyhavior_simulator/src/bin/`.
