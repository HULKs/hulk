# Behavior Tree Simulator

`crates/bevyhavior_simulator` runs the behavior tree from `behavior_node` directly
inside a Bevy simulation. Each simulated robot has its own behavior state and
receives a `WorldState` assembled from simulated perception, field geometry,
game state, and teammate messages.

The simulator applies the resulting motion commands to simplified robot and ball
kinematics. It supports team-message routing, an automatic referee, invariant
checks, and a recorded timeline. It tests behavior decisions and interactions;
it does not validate camera inference, SDK execution, or physical robot stability.

## Run a scenario

From the repository root:

```sh
cargo run -p bevyhavior_simulator --bin behavior_tree_smoke
```

The scenario runs to completion and then opens the timeline viewer. Set
`BEVYHAVIOR_SIMULATOR_NO_VIEWER` to suppress the viewer:

```sh
BEVYHAVIOR_SIMULATOR_NO_VIEWER=1 cargo run -p bevyhavior_simulator --bin behavior_tree_smoke
```

Run the same scenario as a headless Cargo test:

```sh
cargo test -p bevyhavior_simulator --bin behavior_tree_smoke
```

These commands require the workspace's native build dependencies. The viewer
also needs a display and supported graphics adapter. Headless execution skips
the viewer but still builds the crate's dependencies.

The smoke scenario reports success after a goal and failure if no goal occurs
within 2,000 recorded frames. Scenario exit status and invariant failures both
contribute to the result; inspect the output when a run fails.

## Define a scenario

Scenarios live in `crates/bevyhavior_simulator/src/bin/`. Start from
`behavior_tree_smoke.rs` or `mercy_rule_finish.rs`:

1. Define a function taking `&mut App` and annotate it with `#[scenario]`.
2. Add startup systems that spawn `SimulatorRobotBundle` and `SimulatorBall`
   entities and configure `SimulatorGameState`.
3. Add update systems for events and assertions. Use `BehaviorTreeSimulatorSet`
   to order them relative to the simulation stages.
4. Emit `AppExit::Success` or a nonzero `AppExit` when the scenario has a result.
   Include a frame/time limit so a failed condition cannot run forever.

The macro in `crates/scenario/src/lib.rs` creates both the executable entry point
and a headless test. Both install `BehaviorTreeSimulatorPlugin::default()` before
applying the scenario's setup.

`SimulatorRobotBundle::new` takes a team, player number, initial pose, and
`BehaviorParameters`. `default_behavior_parameters()` loads the checked-in base
behavior parameters. Add scenario-specific overrides explicitly.

## Configuration

`BehaviorTreeSimulatorPlugin` configures field dimensions, tick duration,
simulation parameters, communication, automatic refereeing, and the default
processing stages. `SimulationConfig` in `src/config.rs` defines the kinematic
and perception settings. Selected defaults are:

| Setting | Default |
| --- | --- |
| Tick duration | 10 ms |
| `walk_translation_speed` | 2.0 |
| `walk_rotation_speed` | 3.0 |
| `ball_visibility_range` | 4.0 |
| `visibility_field_of_view` | π/2 |
| `ball_friction_per_second` | 0.6 |
| `robot_radius` | 0.16 |
| `kick_radius` | 0.35 |

These are simulation settings, not measured Booster hardware limits.
Use `src/config.rs` and `src/auto_referee.rs` for the complete configuration.

## Inspect a run

`SimulatorTimeline` contains the recorded frames; `SimulatorTimelineMarkers`
adds scenario annotations. Frames include robot world states, behavior results,
motion commands, and invariant violations. The viewer displays the recorded run
after execution rather than controlling a live robot.

Useful implementation entry points:

- `src/behavior_tree_simulator.rs`: plugin, scheduling, scenario execution, and results.
- `src/behavior_runtime.rs`: per-robot behavior state and tree evaluation.
- `src/world_states.rs`: simulated perception and world-state assembly.
- `src/communication.rs`: team-message routing.
- `src/kinematics.rs`: motion-command application and collisions.
- `src/auto_referee.rs`: game-state progression.
- `src/invariant_checks.rs`: checks performed during scenarios.
- `src/timeline_viewer.rs`: recorded-run viewer.
