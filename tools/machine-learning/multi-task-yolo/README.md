# multi-task-yolo

Utilities for assembling, validating, training and exporting shared-backbone YOLO
models with detection, pose and segmentation heads.

## Setup and command help

Use Python 3.13 and `uv`. Run from `tools/machine-learning/multi-task-yolo`:

```bash
uv sync
uv run -m validation.validator --help
uv run -m model.train --help
uv run -m utils.export_hydra --help
uv run -m utils.export_yolo_to_onnx --help
uv run -m utils.model_complexity --help
uv run -m validation.compare_results --help
uv run ruff check src
uv run ruff format src
```

The modules live under `src/`: `model/hydra.py` assembles models,
`validation/validator.py` validates and saves assembled single-head checkpoints,
`model/train.py` trains those checkpoints, and `utils/export_hydra.py` exports them.
`validation/predictor.py` is a local smoke script with a hardcoded image path, not
a general-purpose prediction CLI.

## Model names and paths

Model names use `BACKBONE=fN+HEAD[+HEAD...]`, for example:

```text
yolo26m=f11+yolo26m+yolo26m-pose
```

`f11` selects the number of frozen/shared modules. Head names starting with
`yolo26m`, `yolo26m-pose` and `yolo26m-seg` identify object, pose and segmentation
tasks respectively. A `~suffix` identifies a finetuned head.

Validation and training take repeatable **`--hydra_model_name`** options. Dataset
options also use underscores: `--object_dataset_name`, `--pose_dataset_name`, and
`--segmentation_dataset_name`. Dataset paths are resolved under
`<assets_dir>/datasets`; validation's source checkpoints are resolved under `<assets_dir>`.
Defaults are `assets`, `runs`, `coco.yaml`, `coco-pose.yaml` and `coco.yaml` respectively.
Obtain the datasets and source checkpoints separately; `uv sync` does not provide them.

## Validate and assemble checkpoints before training

```bash
uv run -m validation.validator \
  --hydra_model_name 'yolo26m=f11+yolo26m+yolo26m-pose' \
  --object_dataset_name nao_coco_k1_data.yaml \
  --pose_dataset_name coco-pose.yaml \
  --validate-original
```

The default image size is 640, batch size is 16 and device is the string `-1`.
Use `--device cpu` or another Ultralytics-supported device value explicitly when
needed. `--validate-original` additionally validates source heads.

The command splits multi-head names into single-head validation runs, such as
`runs/val/yolo26m=f11+yolo26m/` and `runs/val/yolo26m=f11+yolo26m-pose/`.
Each assembled run contains a checkpoint named after the run, plus `metrics.json`
and `config.json`. Original-head runs use names such as `runs/val/yolo26m/`.
The validator currently **does not write `metadata.json`**.

## Train assembled single-head checkpoints

Training loads the corresponding checkpoint under
`<runs_dir>/<val_dir>/<single-head-model-name>/<single-head-model-name>.pt`.
Run validation/assembly first, then:

```bash
uv run -m model.train \
  --hydra_model_name 'yolo26m=f11+yolo26m' \
  --object_dataset_name nao_coco_k1_data.yaml \
  --device 0 --epochs 100
```

Defaults include `--runs_dir runs`, `--val_dir val`, `--assets_dir assets`,
`--epochs 100` and `--device -1`. `--device` is passed through as a string to
Ultralytics; it is not parsed into a Python list by this CLI.
There is no `--dev-mode`; request fewer epochs explicitly for a shorter run.
Training initializes Weights & Biases, so configure its account or execution mode
for your environment.

Each training run has a random word suffix, for example
`runs/train/yolo26m=f11+yolo26m~cheek/weights/best.pt`. Record the actual run name;
the suffix is not a stable output directory or a reproducibility seed.
`--do-tuning` invokes tuning first. The current helper subsequently looks for
`best_hyperparameters.yaml` beside the assembled validation checkpoint, rather
than locating the randomly named tuning output directory; it does not copy that
file there. Treat tuning as an incomplete artifact-handling workflow.
`--use-tuned-hyperparameters` applies the parameters
loaded during that invocation's tuning path; it does not independently load a
documented fixed `runs/tune/.../best_hyperparameters.yaml` file.

## Export Hydra

The positional arguments are one or more model names followed by an **output
directory**, not a checkpoint path followed by an output filename. There is no
`--head` option. Non-finetuned heads come from assembled validation checkpoints;
finetuned heads come from training runs selected by their `~suffix`.
The exporter passes the bare backbone name to Ultralytics, rather than prepending
`assets`. Make the same backbone checkpoint available in the working directory
to avoid resolving/downloading a different checkpoint:

```bash
cp assets/yolo26m.pt yolo26m.pt
uv run -m utils.export_hydra \
  'yolo26m=f11+yolo26m+yolo26m-pose' \
  assets/output --with-nv12-layer
```

This attempts to write
`assets/output/yolo26m=f11+yolo26m+yolo26m-pose.onnx`.
Defaults are `--format onnx`, `--imgsz 640`, `--opset 20`, `--device cpu`,
`--runs_dir runs`, `--val_dir val` and `--train_dir train`.
Use the actual finetuned head name to export a trained head, for example
`yolo26m=f11+yolo26m~cheek+yolo26m-pose` if that training run exists.

`--with-nv12-layer` requires an even image size and exports uint8 NV12 input named
`raw_bytes_input` with dynamic half-height/half-width axes and six channels.
Supply the deployed shape when compiling TensorRT.

**Exporter limitations:** `--format pt` uses TorchScript serialization but currently still
names the resulting file `<model-name>.onnx`. The ONNX path returns after the
first model, so invoke once per model rather than assuming a multi-model export
processes every argument.

## Export a single YOLO checkpoint

```bash
mkdir -p assets/output
uv run -m utils.export_yolo_to_onnx \
  assets/yolo26m.pt assets/output/yolo26m-nv12.onnx
```

This exporter takes a checkpoint and a destination **file**. `--subsample` enables
its preprocessing wrapper's subsampling behavior. Its NV12 image dimensions are
dynamic, so TensorRT compilation needs an input shape override.

## Compare saved runs: current artifact gap

```bash
uv run -m validation.compare_results \
  --baseline runs/val/complete-baseline \
  --candidate runs/val/complete-candidate --task detect
```

Replace the example directory names with complete existing runs. This command
requires `metrics.json`, `config.json` **and `metadata.json`** in both
directories. It cannot consume fresh validator output as-is because the producer
does not emit metadata. Use only complete existing run artifacts with genuine
metadata; do not fabricate metadata to make a comparison pass. Resolving the
producer/consumer contract requires a code change.

For complete runs, output defaults to `<candidate>/comparison.json`.
Options include `--task auto|detect|pose`, `--strict-config`, `--primary-metric`
and `--regression-threshold` (default `-0.01`).

## Model complexity

```bash
uv run -m utils.model_complexity assets \
  --checkpoint-name yolo26m.pt --checkpoint-name yolo26m-pose.pt
uv run -m utils.model_complexity runs/train --checkpoint-name best.pt
uv run -m utils.model_complexity \
  --hydra-model-name 'yolo26m=f11+yolo26m+yolo26m-pose'
```

Reports include parameters, MACs, FLOPs (`1 MAC = 2 FLOPs`) and file size.
Checkpoint reports go under `runs/complexity/<checkpoint-name>/report.json`;
assembled-model reports and exports go under `runs/complexity/<model-name>/`.
Use `--json-output <file>` for the combined report.

## Deployment

See `tools/machine-learning/multi-task-yolo/REPRODUCE.md` for the checkpoint and
configuration workflow and `tools/tensorrt-compile/README.md` for compilation.
The detector loads the filename selected by `detection.model_name`, not a hardcoded
`hydra-nv12.onnx`. Its current base configuration selects
`yolo26m-seg=f11+yolo26m~cheek+yolo26m-pose~badge.onnx` in `etc/neural_networks`.
