# multi-task-yolo

Utilities for building, validating, and exporting a Hydra-style multi-task
YOLO model that shares one backbone across detection and pose heads.

## Requirements

- Python `3.13`
- `uv` for dependency management and command execution

## Setup

From this directory (`tools/machine-learning/multi-task-yolo`):

```bash
uv sync
```

This project uses `uv run ...` for all commands.

## Project layout

- `src/model/hydra.py`: Hydra model assembly (shared backbone + per-task heads).
- `src/model/train.py`: Click CLI for single-task YOLO tuning/training.
- `src/validation/validator.py`: validation pipeline for original models and Hydra heads.
- `src/validation/compare_results.py`: compare two saved validation runs.
- `src/validation/predictor.py`: local smoke predictor/visualizer for detection + pose.
- `src/utils/export_yolo_to_onnx.py`: export a single YOLO checkpoint with NV12 preprocessing.
- `src/utils/export_hydra.py`: export Hydra models to ONNX or TorchScript (optional NV12 layer).
- `src/utils/nv12_to_rgb.py`: NV12-to-RGB layer used by export wrappers.

## Common commands

```bash
# Lint
uv run ruff check src

# Format
uv run ruff format src

# Training CLI help
uv run python src/model/train.py --help

# Validation CLI help
uv run -m validation.validator --help

# Compare two validation runs
uv run -m validation.compare_results --help

# Single-task ONNX export help
uv run -m utils.export_yolo_to_onnx --help

# Hydra export help
uv run -m utils.export_hydra --help
```

## Single-task training (`src/model/train.py`)

Finetunes one YOLO model and can optionally run hyperparameter tuning first.

- `--project-tune-dir` and `--project-train-dir` override output roots.
- If omitted, output roots resolve from `--repo-root` to
  `<repo-root>/runs/tune` and `<repo-root>/runs/train`.
- `--device` accepts a comma-separated list and is parsed to `list[int]`
  (example: `--device 0,1`).
- `--dev-mode` is opt-in and switches to short development settings.
- `--do-tuning` is opt-in and runs `model.tune()` before `model.train()`.
- `--use-tuned-hyperparameters` loads
  `runs/tune/<tuning-folder-name>/best_hyperparameters.yaml`.

Examples:

```bash
# Default training
uv run python src/model/train.py

# Fast development run
uv run python src/model/train.py --dev-mode

# Multi-GPU training
uv run python src/model/train.py --device 0,1

# Tune then train
uv run python src/model/train.py --do-tuning
```

## Validation (`src/validation/validator.py`)

Runs Ultralytics validation for Hydra heads and can optionally validate the
original source checkpoints first.

- Default checkpoints:
  - `--backbone assets/yolo26m.pt`
  - `--detection-model assets/yolo26m.pt`
  - `--pose-model assets/yolo26m-pose.pt`
- Default datasets:
  - `--detection-data assets/datasets/coco.yaml`
  - `--pose-data assets/datasets/coco-pose.yaml`
- `--validate-original` enables baseline validation of the original task
  models before multi-task validation.

Example:

```bash
uv run -m validation.validator \
  --backbone assets/yolo26m.pt \
  --detection-model assets/yolo26m.pt \
  --pose-model assets/yolo26m-pose.pt \
  --detection-data assets/datasets/coco.yaml \
  --pose-data assets/datasets/coco-pose.yaml \
  --validate-original
```

Validation outputs are saved under `runs/val/...` and include:

- `metrics.json`
- `metadata.json`
- `config.json`

## Compare validation runs (`src/validation/compare_results.py`)

Compares two saved validation run directories of the same task type and writes
a JSON report.

- Required inputs: `--baseline <run_dir>` and `--candidate <run_dir>`.
- By default, writes output to `<candidate>/comparison.json`.
- Supports `--task auto|detect|pose`, strict config checks via
  `--strict-config`, custom primary metric, and regression threshold.

Example:

```bash
uv run -m validation.compare_results \
  --baseline runs/val/yolo26m \
  --candidate runs/val/yolo26m-pose_yolo26m \
  --task auto
```

## Export utilities

### Export single YOLO to ONNX (`src/utils/export_yolo_to_onnx.py`)

Wraps a YOLO checkpoint with an NV12 preprocessing layer and exports ONNX.

```bash
uv run -m utils.export_yolo_to_onnx \
  assets/yolo26m.pt \
  assets/output/yolo26m-nv12.onnx
```

Use `--subsample` to enable chroma subsampling behavior in the wrapper.

### Export Hydra model (`src/utils/export_hydra.py`)

Builds a Hydra model from a backbone checkpoint plus one or more heads,
then exports ONNX (`--format onnx`) or TorchScript (`--format pt`).

- Repeat `--head NAME=MODEL_PATH` for each task head.
- Optional `--with-nv12-layer` prepends NV12 preprocessing before export.
- When `--with-nv12-layer` is enabled, `--imgsz` must be even.

Examples:

```bash
# ONNX export
uv run -m utils.export_hydra \
  assets/yolo26m.pt \
  --head detection=assets/yolo26m.pt \
  --head pose=assets/yolo26m-pose.pt \
  assets/output/hydra.onnx

# TorchScript export with NV12 input wrapper
uv run -m utils.export_hydra \
  assets/yolo26m.pt \
  --head detection=assets/yolo26m.pt \
  --head pose=assets/yolo26m-pose.pt \
  assets/output/hydra-nv12.pt \
  --format pt \
  --with-nv12-layer
```

## Local predictor note

`src/validation/predictor.py` contains a local smoke workflow. Its `main()`
uses a hardcoded example image path under `assets/datasets/...` and is not a
general-purpose CLI entrypoint.
