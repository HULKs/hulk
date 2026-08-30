# multi-task-rf-detr

RF-DETR fine-tuning pipeline for HULKs RoboCup perception, as the team moves from NAO toward
the Booster platform. RF-DETR is a DINOv2-backed detection transformer whose backbone is
reusable across tasks (detection now; segmentation/pose planned). Models are fine-tuned
locally and exported to ONNX for the robot inference stack (ONNX → `ort` / TensorRT on
Jetson Orin NX).

## Requirements

- Python `3.12` (rfdetr / PyTorch are validated on 3.12; note the sibling `multi-task-yolo`
  uses 3.13 — see the deviation note below)
- `uv` for dependency management and command execution
- NVIDIA GPU + CUDA 12.4 for training (CPU is fine for ONNX export/validation)

## Setup

From this directory (`tools/machine-learning/multi-task-rf-detr`):

```bash
uv sync
```

This installs the pinned, validated stack: PyTorch `2.5.1+cu124`, `rfdetr[train,loggers]`
`1.7.1`, supervision, albumentations, onnx tooling. `torch`/`torchvision` are sourced from
the CUDA 12.4 index declared in `pyproject.toml`.

## Project layout

- `src/data/convert.py` — NAO YOLO → RF-DETR COCO conversion (lossless, round-trip checked).
- `src/data/verify.py` — COCO dataset verification (boxes, image↔label pairing, class counts).
- `src/training/config.py` — YAML config loader (dataclasses; resolution-÷56 validation).
- `src/training/train.py` — config-driven RF-DETR detection training.
- `src/export/to_onnx.py` — export a checkpoint to static-batch ONNX (TensorRT-safe).
- `src/export/validate.py` — validate the ONNX (structural + I/O contract + CPU/CUDA parity).
- `src/smoke.py` — environment smoke test.
- `assets/` — training configs: `detection.yaml`, `segmentation.yaml`, `pose_experimental.yaml`.
- `docs/` — `DATA_REPORT.md` (dataset audit) and `parity_report.md` (ONNX hand-off contract).

## Common commands

```bash
# Lint / format
uv run ruff check src
uv run ruff format src

# Dataset: convert NAO YOLO → COCO, then verify
uv run -m data.convert
uv run -m data.verify --config assets/detection.yaml

# Train detection (use --dry-run to validate config without a GPU)
uv run -m training.train --config assets/detection.yaml --dry-run
uv run -m training.train --config assets/detection.yaml

# Export + validate ONNX
uv run -m export.to_onnx --config assets/detection.yaml
uv run -m export.validate --config assets/detection.yaml
```

## Status

Detection pipeline validated end-to-end (convert → train → export → ONNX parity) on a trial
checkpoint. Segmentation and pose are planned but data-blocked (no mask/keypoint annotations
yet); see `docs/DATA_REPORT.md`. The full-dataset baseline run is tracked separately.

## Datasets

Datasets are **not committed** (size + the team's DVC convention) — they are referenced, not
shipped. The primary corpus is the labelled NAO detection set (7 classes: Ball, GoalPost,
LSpot, PenaltySpot, Robot, TSpot, XSpot). For access, ask the HULKs members. See `REPRODUCE.md`.

## Why Python 3.12 (deviation from the 3.13 sibling)

`rfdetr 1.7.1` + the CUDA PyTorch stack are validated on Python 3.12 in this project. The
`pyproject.toml` pins `requires-python = ">=3.12,<3.13"` accordingly. Everything else
(uv, `pyproject.toml`, `ruff.toml`, `src/` layout) mirrors `multi-task-yolo`.
