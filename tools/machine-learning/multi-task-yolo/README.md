# multi-task-yolo

Utilities for building and evaluating a Hydra-style multi-task YOLO model
that shares one backbone across detection and pose heads.

## What is in this repo

- `src/model/hydra.py`: assembles a shared-backbone, multi-head model.
- `src/validation/validator.py`: validates original YOLO models and Hydra heads.
- `src/validation/compare_results.py`: compares two saved validation runs.
- `src/validation/predictor.py`: runs inference and saves a combined visualization.
- `src/utils/export_yolo_to_onnx.py`: exports a YOLO checkpoint to ONNX with NV12 preprocessing.

## Requirements

- Python `3.13`
- `uv` for dependency management and command execution

## Setup

From this directory (`tools/machine-learning/multi-task-yolo`):

```bash
uv sync
```

Use `uv run -m ...` for module entrypoints.

## Common commands

```bash
# Lint
uv run ruff check src

# Format
uv run ruff format src

# Validation CLI help
uv run -m validation.validator --help

# Compare two validation runs
uv run -m validation.compare_results --help

# ONNX export CLI help
uv run -m utils.export_yolo_to_onnx --help

# Single-task training CLI help
uv run python src/model/train.py --help
```

## Single-task training CLI

`src/model/train.py` is a Click CLI for finetuning a single task model.

- Tune and train output roots are configurable via `--project-tune-dir` and `--project-train-dir`.
- Paths default dynamically from `--repo-root` to `<repo-root>/runs/tune` and `<repo-root>/runs/train`.
- Device accepts a comma-separated list, e.g. `--device 0,1`.
- Development mode is off by default and only runs when `--dev-mode` is supplied.
- Tuning is off by default and only runs when `--do-tuning` is supplied.
- Tuned hyperparameters can be loaded without tuning via `--use-tuned-hyperparameters`.

Examples:

```bash
# Full settings (defaults)
uv run python src/model/train.py

# Fast dev run
uv run python src/model/train.py --dev-mode

# Use multiple GPUs
uv run python src/model/train.py --device 0,1

# Run tuning before training
uv run python src/model/train.py --do-tuning
```

## Notes

- Validation outputs are written under `runs/val/...` with `metrics.json`, `metadata.json`, and `config.json`.
- `src/model/train.py` supports optional tuned hyperparameter loading from `runs/tune/<name>/best_hyperparameters.yaml`.
- `src/validation/predictor.py` `main()` is a local smoke example and uses a hardcoded image path.
