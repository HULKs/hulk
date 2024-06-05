# Behavior Simulator

A simplified simulator which can be used for manual or automatic testing of behavior in a defined scenario.

# Usage

The simulator has two modes `run` and `serve`.

## Run

In this mode, when the simulation is finished, the simulator exits.
It returns an error if the robotics code encountered a problem or if the scenario file generated an error.

```sh
./pepsi run --target behavior_simulator -- run tests/behavior/golden_goal.lua
```

## Serve

In this mode, after the simulation is finished, the simulator opens a commmunication server for use with e.g. [Twix](./twix.md).

```sh
./pepsi run --target behavior_simulator -- serve tests/behavior/golden_goal.lua
```

Within twix, connect to `localhost` and open a `Behavior Simulator` panel.
This panel has a timeline slider for scrubbing through the scenario.

!!! info

    To see all robots on the map panel, make sure to enable the behavior simulator overlay. Otherwise only the selected robot is shown.

# Scenario Development

Scenario files are written in [Lua](https://www.lua.org/) and can be found at `tests/behavior/`.
The file is interpreted once on startup and can define functions like `on_cycle` which are then called repeatedly by the simulator.

Common actions within these callbacks include changing game states, moving the ball around, or penalizing robots.
See [demonstration.lua](https://github.com/HULKs/hulk/blob/main/tests/behavior/demonstration.lua) for examples.
