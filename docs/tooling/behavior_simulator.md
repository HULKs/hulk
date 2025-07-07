# Bevyhavior Simulator

A simplified simulator which can be used for manual or automatic testing of behavior in a defined scenario.

# Usage

```sh
./pepsi run --bin golden_goal
```

After the simulation is finished, the simulator opens a commmunication server for use with e.g. [Twix](./twix.md).
It returns an error if the robotics code encountered a problem or if the scenario file generated an error.


Within twix, connect to `localhost` and open a `Behavior Simulator` panel.
This panel has a timeline slider for scrubbing through the scenario.

!!! info

    To see all robots on the map panel, make sure to enable the behavior simulator overlay. Otherwise only the selected robot is shown.

# Scenario Development

Scenario files can be found at `crates/bevyhavior_simulator/src/bin/`.