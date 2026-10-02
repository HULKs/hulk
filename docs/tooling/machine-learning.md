# Machine Learning

The repository contains separate dataset, vision-model and motion-policy tools. Python projects use Python 3.13 and `uv`; run their commands from the tool's directory.

## Vision models

- **Hydra / multi-task YOLO:** `tools/machine-learning/multi-task-yolo` assembles shared-backbone detection, pose and segmentation models. Its [README](https://github.com/HULKs/hulk/blob/main/tools/machine-learning/multi-task-yolo/README.md) describes current model-name CLI arguments, validation artifacts and export naming. The [reproduction guide](https://github.com/HULKs/hulk/blob/main/tools/machine-learning/multi-task-yolo/REPRODUCE.md) explains training prerequisites and deployment configuration. Dataset access and historical deployed checkpoints must be obtained from the team.
- **XFeat / LighterGlue:** `tools/machine-learning/xfeat-export` exports NV12 feature extraction and matching models. Its [README](https://github.com/HULKs/hulk/blob/main/tools/machine-learning/xfeat-export/README.md) documents shapes, fused visual-odometry state and the KITTI benchmark.

For Hydra, start by inspecting the current interfaces:

```bash
cd tools/machine-learning/multi-task-yolo
uv sync
uv run -m validation.validator --help
uv run -m model.train --help
uv run -m utils.export_hydra --help
```

The validator creates assembled single-head checkpoints used by training. Training emits randomly suffixed run names; exports take model names and an output directory, not `--head` arguments or a destination filename.
The saved-run comparison tool currently requires `metadata.json`, which the validator does not produce. Fresh validation output cannot be compared by that tool without resolving this artifact-contract gap.

## Dataset annotation

Annotato labels image datasets and synchronizes them with a configured host. From `tools/annotato`, provide `annotato.toml` matching the configuration types in `tools/annotato/src/user_toml.rs`:

```bash
cargo run -- --help
cargo run -- data --help
cargo run -- label my-dataset --offline
```

Offline labeling requires `current/<dataset-name>/images` and `current/<dataset-name>/data.json` locally. Without `--offline`, the tool downloads a missing dataset and uploads it after labeling. Host credentials and dataset access are environment-specific.

## Motion-policy training

Konerl in `tools/machine-learning/konerl` integrates K1 tasks with MJLab. See its [README](https://github.com/HULKs/hulk/blob/main/tools/machine-learning/konerl/README.md) for the registered standing task and limitations of the checked-in velocity-training example. Training hardware, exported weights and the production motion-inference policies are separate setup concerns.

## Compile and deploy ONNX models

Run the following host commands from the repository root, targeting a provisioned K1 with the matching runtime:

```bash
./pepsi hulk stop 42
./pepsi tensor-rt-compile 'etc/neural_networks/<model-name>.onnx' 42 -- --raw_bytes_input 272,320,6
```

This example fixes a dynamic NV12 input at 640×544 pixels; choose the shape required by the model and deployed image pipeline. Static models, such as the fused XFeat/LighterGlue export, do not need shape overrides.
See the [TensorRT guide](https://github.com/HULKs/hulk/blob/main/tools/tensorrt-compile/README.md) for upload behavior, compilation checks and cache deployment.

The detector loads `detection.neural_networks_folder` joined with `detection.model_name`. The current base model name is `yolo26m-seg=f11+yolo26m~cheek+yolo26m-pose~badge.onnx`; `hydra-nv12.onnx` is not a hardcoded runtime filename.
The fused VO model is selected by `stereo_visual_odometry.neural_network`, defaulting to `etc/neural_networks/xfeat-lighterglue.onnx`. Set the appropriate parameter layer to the artifact actually exported, then upload with Pepsi.
