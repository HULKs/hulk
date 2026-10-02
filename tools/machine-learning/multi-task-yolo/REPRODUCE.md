# Hydra training, export and deployment

The current detector configuration in `etc/parameters/base/detection.json5` selects:

```text
yolo26m-seg=f11+yolo26m~cheek+yolo26m-pose~badge.onnx
```

That name identifies a segmentation-model backbone and finetuned detection/pose
heads. Exact reproduction needs the original checkpoints, datasets, training
settings and software environment. Obtain those artifacts and dataset access from
the team; rerunning training with a random suffix does not recreate those weights.

The earlier recipe described a `yolo26m` backbone, a detection head finetuned on
`nao_coco_k1_data.yaml`, and an off-the-shelf pose head. The following is a current
CLI recipe for **that model family**, not a claim to reproduce today's configured
deployed weights. Read `tools/machine-learning/multi-task-yolo/README.md` for the
current saved-run and exporter limitations.

## 1. Prepare assets

Run Python commands from `tools/machine-learning/multi-task-yolo`:

```bash
cd tools/machine-learning/multi-task-yolo
uv sync
```

Use Python 3.13. Provide `assets/yolo26m.pt`, `assets/yolo26m-pose.pt`, and dataset
YAML files under `assets/datasets`. If your datasets are actually in `/opt/data`
and `assets/datasets` does not already exist, you can link that directory:

```bash
ln -s /opt/data assets/datasets
```

Dataset paths inside the YAML must resolve in your environment.

## 2. Validate and create assembled checkpoints

Training requires the assembled single-head validation checkpoint, so validation
comes before training:

```bash
uv run -m validation.validator \
  --hydra_model_name 'yolo26m=f11+yolo26m+yolo26m-pose' \
  --object_dataset_name nao_coco_k1_data.yaml \
  --pose_dataset_name coco-pose.yaml \
  --validate-original --device 0
```

Defaults include `--assets_dir assets`, `--runs_dir runs`, `--imgsz 640` and
`--batch 16`. The assembled detection checkpoint is
`runs/val/yolo26m=f11+yolo26m/yolo26m=f11+yolo26m.pt`; the pose checkpoint follows
the same naming convention with `yolo26m-pose`.
The validator saves `metrics.json` and `config.json`, but not `metadata.json`.
Consequently `validation.compare_results` cannot compare these new runs as-is:
it requires metadata in both input directories. This remains a code-level workflow gap.

## 3. Train the detection head

```bash
uv run -m model.train \
  --hydra_model_name 'yolo26m=f11+yolo26m' \
  --object_dataset_name nao_coco_k1_data.yaml \
  --device 0 --epochs 100
```

The training command uses the assembled checkpoint above and initializes Weights
& Biases. Configure W&B for your environment. Tuning is opt-in via `--do-tuning`;
there is no `--dev-mode`. Defaults include `--runs_dir runs` and `--val_dir val`.

Record the actual randomly suffixed output directory and retain
`weights/best.pt`. For subsequent commands, replace the placeholder suffix:

```bash
DETECTION_RUN='yolo26m=f11+yolo26m~<actual-training-suffix>'
DETECTION_HEAD="${DETECTION_RUN#*+}"
MODEL_NAME="yolo26m=f11+${DETECTION_HEAD}+yolo26m-pose"
```

The exporter resolves the finetuned detection checkpoint directly from
`runs/train/$DETECTION_RUN/weights/best.pt`; no copy to a fixed
`assets/yolo26m-tuned.pt` filename is required.

## 4. Export and check the artifact

The exporter loads the bare backbone name through Ultralytics, not from `assets`.
Place the intended backbone checkpoint in the working directory too:

```bash
cp assets/yolo26m.pt yolo26m.pt
uv run -m utils.export_hydra "$MODEL_NAME" assets/output --with-nv12-layer
```

The last positional argument is a directory. The intended output file is
`assets/output/$MODEL_NAME.onnx`, not `hydra-nv12.onnx`.
Defaults are `--format onnx`, `--imgsz 640`, `--opset 20` and `--device cpu`.
The NV12 input is named `raw_bytes_input` and has dynamic half-height/half-width
dimensions. Confirm that export completes and the artifact is usable before
deploying; the current exporter is not an end-to-end validated reproduction workflow.

After a successful export, copy that file into the repository's model directory:

```bash
cp "assets/output/$MODEL_NAME.onnx" "../../../etc/neural_networks/$MODEL_NAME.onnx"
```

In the intended deployment parameter layer, set `detection.model_name` to that
filename and `detection.neural_networks_folder` to `etc/neural_networks`.
Uploading a new model file alone does not change which model the detector loads.
To export the currently configured model instead, obtain the exact matching
backbone and both finetuned run artifacts and use its model name; the illustrative
training run above is not a replacement for those checkpoints.

## 5. Compile TensorRT on a K1

Return to the repository root. With the provisioned `hulk` runtime container
running and robot 42 reachable:

```bash
cd ../../..
./pepsi hulk stop 42
./pepsi tensor-rt-compile "etc/neural_networks/$MODEL_NAME.onnx" 42 -- --raw_bytes_input 272,320,6
```

This example compiles for a 640×544 NV12 image. The 640-square export trace does
not establish the deployment image dimensions; select the shape used by the
actual pipeline. See `tools/tensorrt-compile/README.md` for static models, manual
compilation, upload behavior and failure handling.

## 6. Deploy the configured artifact and cache

```bash
./pepsi upload 42
```

Upload transfers the robot executable and `etc`, including parameter layers,
neural-network files and caches, then starts the HULK service. Keep the model,
selected parameter filename and target-compatible TensorRT cache together.
