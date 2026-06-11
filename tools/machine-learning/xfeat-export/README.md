# XFeat ONNX Export

Exports XFeat and LighterGlue to fixed-contract ONNX models for later TensorRT conversion.

```bash
uv run export-xfeat-onnx models/xfeat.onnx --height 448 --width 544 --keypoints 512
uv run export-xfeat-onnx models/xfeat-b2.onnx --height 448 --width 544 --keypoints 512 --batch-size 2
uv run export-lighterglue-onnx models/lighterglue.onnx --keypoints 512
uv run export-xfeat-lighterglue-onnx models/xfeat-lighterglue.onnx --height 448 --width 544 --keypoints 512
uv run export-xfeat-lighterglue-onnx ../../../etc/neural_networks/xfeat-lighterglue.onnx --height 448 --width 544 --keypoints 512
```

`export-xfeat-onnx` takes NV12 input as `uint8` shaped `(height / 2, width / 2, 6)`, for example `(224, 272, 6)` for a `544 x 448` image, and embeds the GPU NV12-to-RGB conversion layer from `../multi-task-yolo/src/utils/nv12_to_rgb.py` into the exported graph.
With `--batch-size`, the XFeat input is `uint8` shaped `(batch_size, height / 2, width / 2, 6)`.
It returns normalized keypoints, descriptors, scores, and valid masks. The keypoints use the LighterGlue normalization `(keypoint - [width, height] / 2) / (max(width, height) / 2)`.
`export-lighterglue-onnx` expects those normalized keypoints directly, so the exported LighterGlue model does not take image-size inputs.

`export-xfeat-lighterglue-onnx` fuses current-frame extraction and matching for visual odometry. It takes two zero-copy NV12 inputs named `current_left` and `current_right`, each shaped `(height / 2, width / 2, 6)`, plus the previous-left feature state: `previous_left_keypoints` shaped `(512, 2)`, `previous_left_descriptors` shaped `(512, 64)`, and `previous_left_valid` shaped `(512)`. It returns current-left/right keypoints, descriptors, valid masks, current-left-to-current-right stereo matches, previous-left-to-current-left temporal matches, and the reverse match directions for diagnostics. The Rust VO node only extracts the CPU-required outputs: current-left state, current-right keypoints, stereo matches, and temporal matches.

For TensorRT compilation of the static fused model, run from the repository root:

```bash
cargo run --release -p tensorrt-compile -- \
  --cache-path etc/neural_networks \
  etc/neural_networks/xfeat-lighterglue.onnx
```

The fused exporter validates the generated ONNX model with `onnx.checker` and ONNX Runtime, including a two-step state feedback smoke test.

The ROS-Z `stereo_visual_odometry` node is disabled by default. Enable `stereo_visual_odometry.enable` only after `stereo_visual_odometry.neural_networks_folder` contains `stereo_visual_odometry.model_name`, which defaults to `etc/neural_networks/xfeat-lighterglue.onnx`.

To benchmark the fused visual odometry model on KITTI odometry data, place the official `data_odometry_gray.zip`, `data_odometry_calib.zip`, and `data_odometry_poses.zip` archives in `../datasets` relative to the repository root and run:

```bash
cargo bench -p stereo_visual_odometry --bench kitti_odometry
KITTI_MAX_FRAMES=200 KITTI_SEQUENCES=00 cargo bench -p stereo_visual_odometry --bench kitti_odometry
```

The KITTI benchmark reports PNG/ZIP decode and NV12 conversion separately from the timed visual odometry runtime.
