# Reproduce deployed Hydra model (finetuned detection + OTS pose)

This guide reproduces the multi-head model that was deployed on the robot:

- shared backbone: `yolo26m`
- detection head: finetuned on `nao_coco_k1_data.yaml`
- pose head: off-the-shelf `yolo26m-pose`

If you need dataset access/details, ask the hulk members
(for example @alexschmander or @knoellle).

Note: CLI command blocks that omit defaulted arguments end with
`# implied defaults:` comments listing those implicitly applied values.

## Step overview

1. [Prepare prerequisites](#step-0-prepare-prerequisites)
2. [Fine-tune the detection model](#step-1-fine-tune-the-detection-model)
3. [Validate original and Hydra models](#step-2-validate-original-and-hydra-models)
4. [Export Hydra to NV12 ONNX](#step-3-export-hydra-to-nv12-onnx)
5. [Compile TensorRT engine cache](#step-4-compile-tensorrt-engine-cache)
6. [Deploy to robot](#step-5-deploy-to-robot)

## Step 0: Prepare prerequisites

Run Steps 0-4 from this directory:

```bash
cd tools/machine-learning/multi-task-yolo
```

Required:

- Python `3.13`
- `uv`
- dataset YAML (default expected by training CLI):
  `/opt/data/nao_coco_k1_data.yaml`
- model checkpoints:
  - `assets/yolo26m.pt` (base checkpoint)
  - `assets/yolo26m-pose.pt` (pose checkpoint)

Install dependencies:

```bash
uv sync
```

Link the datasets folder into `assets/datasets`:

```bash
ln -sT /opt/data assets/datasets/
```

## Step 1: Fine-tune the detection model

Train detection on your custom dataset:

```bash
uv run python src/model/train.py \
  --base-model-path assets/yolo26m.pt \
  --data assets/datasets/nao_coco_k1_data.yaml \
  --device 0

# implied defaults:
#   --repo-root <cwd> (resolved absolute path)
#   --project-tune-dir <repo-root>/runs/tune
#   --project-train-dir <repo-root>/runs/train
#   --tuning-folder-name yolo26m-tuning
#   --training-folder-name yolo26m-tuned
#   --do-tuning false
#   --use-tuned-hyperparameters false
#   --dev-mode false
```

Training output:

- `runs/train/yolo26m-tuned/weights/best.pt`

Copy the trained model into `assets/` for the next steps:

```bash
cp runs/train/yolo26m-tuned/weights/best.pt assets/yolo26m-tuned.pt
```

## Step 2: Validate original and Hydra models

Run validation for original models and Hydra heads:

```bash
uv run -m validation.validator \
  --foundation assets/yolo26m-tuned.pt \
  --detection-model assets/yolo26m-tuned.pt \
  --pose-model assets/yolo26m-pose.pt \
  --detection-data assets/datasets/nao_coco_k1_data.yaml \
  --pose-data assets/datasets/coco-pose.yaml \
  --validate-original

# implied defaults:
#   --imgsz 640
#   --batch 16
#   --device None (let's pytorch choose the device)
```

Validation outputs are written under `runs/val/...` and include:

- `metrics.json`
- `metadata.json`
- `config.json`

Optional: compare saved validation runs:

```bash
uv run -m validation.compare_results \
  --baseline runs/val/yolo26m \
  --candidate runs/val/yolo26m-tuned_yolo26m-tuned \
  --task detect

# implied defaults:
#   --output <candidate>/comparison.json
#   --strict-config false
#   --primary-metric None (task default is used)
#   --regression-threshold -0.01
```

```bash
uv run -m validation.compare_results \
  --baseline runs/val/yolo26m-pose \
  --candidate runs/val/yolo26m-tuned_yolo26m-pose \
  --task pose

# implied defaults:
#   --output <candidate>/comparison.json
#   --strict-config false
#   --primary-metric None (task default is used)
#   --regression-threshold -0.01
```

## Step 3: Export Hydra to NV12 ONNX

Export the Hydra model directly to the deployment filename/location expected by
the runtime:

```bash
uv run -m utils.export_hydra \
  assets/yolo26m-tuned.pt \
  --head detection=assets/yolo26m-tuned.pt \
  --head pose=assets/yolo26m-pose.pt \
  ../../../etc/neural_networks/hydra-nv12.onnx \
  --with-nv12-layer

# implied defaults:
#   --imgsz 640
#   --opset 20
#   --format onnx
#   --device cpu
```

Why these flags matter:

- `--with-nv12-layer` makes the model accept nv12 image bytes as input directly.
- the deployed object detection node expects
  `etc/neural_networks/hydra-nv12.onnx` by name.

## Step 4: Compile TensorRT engine cache

Compile TensorRT cache artifacts for the exported ONNX model.

Detailed instructions are in:

- [`tools/tensorrt-compile/README.md`](../../tensorrt-compile/README.md)

## Step 5: Deploy to robot

From repository root:

```bash
cd ../../../
./pepsi upload <ROBOT_NUMBER_OR_IP>
```

`pepsi upload` transfers binary + configuration + `etc/neural_networks`
artifacts to the robot and restarts HULK service.
