# Simulator

The experimental Bevy/MuJoCo K1 simulator implementation lives on the [`motion-inference-simulator` branch in Alex's HULK repository](https://github.com/alexschmander/hulk/tree/motion-inference-simulator).
It is **not available on HULK main**: main has neither the `simulator` launcher nor the `tools/simulate` crate. Check out the development branch before using the commands below.

Ball perception, automatic tuning, the standalone tuner, and the Twix optimization monitor described here are **experimental worktree additions**. They are absent from the currently tracked Alex branch and may be in progress or uncommitted remotely. Checking out that branch alone does not provide them; they require the matching implementation worktree.

## Run the local simulator

From the repository root of the development checkout, with its `simulator` launcher:

```sh
./simulator
```

The branch launcher configures MuJoCo 3.9.0. Set `ORT_DYLIB_PATH` to a compatible ONNX Runtime shared library unless the launcher finds `/usr/lib/libonnxruntime.so`; automatic runtime downloading is a worktree addition. Download the K1 models in `etc/neural_networks` with Git LFS before starting the motion stack.
The interactive UI requires a display and a Bevy-supported graphics adapter. It starts paused; wait for motion-model initialization, then press **Run**.

| Mode | Command | Development scope and availability |
| --- | --- | --- |
| Default | `./simulator` | One controlled K1 with production behavior/motion nodes and ground-truth ball/localization inputs; additional robots are passive physical objects. |
| Ball perception (worktree addition) | `./simulator --ball-perception` | Requires matching experimental source. Synthetic camera detections with configurable noise/false positives, production kinematics, camera geometry, odometry and ball filtering. Visual localization and rendered camera inference are not launched. |
| External stack | `./simulator --no-robotics` | Interactive scene, sensors and raw command receiver without launching robotics nodes. Add `--ball-perception` for measured sensor/detection publication. |
| Automatic tuning (worktree addition) | `./simulator --tune-ball-filter logs/ball-tuning` | Requires matching experimental source. Headless physical capture, exact offline filter replay, parameter search and separate holdout evaluation. Use a new output directory. |

Connect Twix from another terminal:

```sh
./twix /simulator/robot --router tcp/127.0.0.1:7447
```

With the matching experimental Twix worktree, add its **Ball-filter optimization** panel for tuning progress and connect to
the optimizer's separate local endpoint `tcp/127.0.0.1:7448`. Add
`--keep-tuning-open` to the tuning command to inspect the final result until Ctrl-C.
See [Twix](twix.md#panels-and-keybindings) for the monitor's connection and scope.

The default local router listens on loopback. The simulator has manual game-state controls, no automatic referee or routed teammate communication, and one controlled robot per router.
See the [development branch's simulator README](https://github.com/alexschmander/hulk/blob/motion-inference-simulator/tools/simulate/README.md) for branch configuration and native-runtime setup. The `tools/simulate/README.md` included in this docs-only update also records experimental worktree workflows; its source paths refer to that development checkout or matching worktree, not main.

## Alex's development branch

To check it out from an existing HULK clone, add the `alex` remote if it is not already configured, then fetch and switch to the branch:

```sh
git remote add alex https://github.com/alexschmander/hulk.git
git fetch alex
git switch --track alex/motion-inference-simulator
```

Use the simulator documentation and `./simulator --help` in the selected branch for its available features. The tuning and perception worktree additions above are not supplied by the currently tracked remote branch.

The [behavior tree simulator design](behavior_tree_simulator_design.md) describes the proposed behavior-testing architecture. It is a design document, rather than a setup guide for a released simulator.
