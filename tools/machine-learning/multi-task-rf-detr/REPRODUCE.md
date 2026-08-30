# Reproduce the RF-DETR detection baseline

This guide reproduces the RF-DETR detection model for HULKs RoboCup perception:

- backbone: DINOv2 (`RFDETRSmall`)
- task: object detection on the NAO dataset (7 classes)
- resolution: `448` (real-time headroom on Jetson Orin NX); export: static-batch ONNX

If you need dataset access/details, ask the HULKs members.

## Step overview

1. [Prepare prerequisites](#step-0-prepare-prerequisites)
2. [Convert NAO YOLO → COCO](#step-1-convert-nao-yolo--coco)
3. [Verify the dataset](#step-2-verify-the-dataset)
4. [Train detection](#step-3-train-detection)
5. [Export to ONNX](#step-4-export-to-onnx)
6. [Validate ONNX + hand-off contract](#step-5-validate-onnx)

## Step 0: Prepare prerequisites

Run all steps from this directory:

```bash
cd tools/machine-learning/multi-task-rf-detr
uv sync
```

Required:

- Python `3.12`, `uv`, an NVIDIA GPU + CUDA `12.4` (for training; CPU is fine for export/validate)
- NAO dataset in YOLO format under `data/nao_dataset/` plus `data/nao_data.yaml`
  (`images/{train,val}`, `labels/{train,val}`; 7 classes). Datasets are not committed — ask the
  HULKs members for access.

## Step 1: Convert NAO YOLO → COCO

```bash
uv run -m data.convert
# implied defaults: --src data/nao_dataset --names data/nao_data.yaml --out data/processed/coco_format
```

Lossless: round-trip < 0.05 px, and output annotation counts / per-class distributions match
the raw labels exactly. Background (empty-label) frames are kept as negatives.

## Step 2: Verify the dataset

```bash
uv run -m data.verify --config assets/detection.yaml
```

## Step 3: Train detection

```bash
uv run -m training.train --config assets/detection.yaml
# RFDETRSmall, res 448, batch 8 x grad_accum 2; 50-epoch ceiling, early-stop patience 10
# checkpoints -> models/rf-detr-small-det/
```

Tip: `--dry-run` validates the config and resolved kwargs without a GPU.

## Step 4: Export to ONNX

```bash
uv run -m export.to_onnx --config assets/detection.yaml
# -> exports/rf-detr-det-448.onnx  (static batch, simplified, structurally checked)
```

## Step 5: Validate ONNX

```bash
uv run -m export.validate --config assets/detection.yaml
# structural + I/O contract + CPU/CUDA parity; writes docs/parity_report.md
```

## Notes

- Segmentation/pose configs exist under `assets/` but are **data-blocked** (no mask/keypoint
  labels yet) — they are placeholders for future tasks.
- The `football-field-keypoints` subset is **excluded** (broadcast-soccer domain mismatch — wrong
  pixel distribution for the robot camera).
- Deployment: hand `exports/rf-detr-det-448.onnx` + `docs/parity_report.md` to the robot inference
  stack (`ort` + TensorRT EP on Jetson Orin NX). FP16-TRT accuracy is validated on-robot.
