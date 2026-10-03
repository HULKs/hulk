# Simulators

## Behavior tree simulator

The current workspace contains `crates/bevyhavior_simulator`. It runs the robot's
behavior tree with simulated world state, robot movement, ball motion, team
communication, and an automatic referee. It records a timeline for inspection.

Run the smoke scenario from the repository root:

```sh
cargo run -p bevyhavior_simulator --bin behavior_tree_smoke
```

The executable opens a timeline viewer after the scenario ends. For headless
execution and scenario tests, see the [behavior tree simulator guide](behavior_tree_simulator_design.md).
This simulator approximates body movement and perception; it does not execute
camera inference or learned motion policies.

## Alex's development branch

The separate MuJoCo-based K1 simulator is experimental work on
[Alex's `motion-inference-simulator` branch](https://github.com/alexschmander/hulk/tree/motion-inference-simulator).
The root `simulator` launcher is absent from this checkout. Use a separate clone
or worktree for that development branch, and follow the README and command help
in the selected source revision.

To obtain the branch in an existing clone, add the remote if it is not already
configured, then create a worktree:

```sh
git remote add alex https://github.com/alexschmander/hulk.git
git fetch alex
git worktree add ../hulk-motion-simulator alex/motion-inference-simulator
```

Available native libraries, model assets, launch options, and implemented modes
must match that checkout. Its instructions do not establish features of the
current robot stack or the behavior tree simulator above.
