# Detector artifact and deployment

The detector selects its model using `neural_networks_folder` and `model_name`
in `etc/parameters/base/detection.json5`, with location/robot parameter overrides.
The current base artifact is:

```text
yolo26m~hslvision=f17+yolo26m~hslvision+yolo26m~hslvision~jail.onnx
```

Fetch the repository's model assets with `git lfs pull`. Exact retraining requires
the original checkpoints, datasets, training settings, and export implementation;
obtain those from the team. A model filename alone is not a reproduction recipe.

## Required runtime contract

`crates/nodes/detection/src/lib.rs` expects:

- An NV12 byte input named `raw_bytes_input`, shaped as
  `[image_height / 2, image_width / 2, 6]`.
- Two float outputs named `hslvision_output` and `nao_output`, each `[1, 300, 6]`.
- Per-candidate bounding-box coordinates, confidence, and class index matching
  `crates/types/src/object_detection.rs` and the detector's class mappings.

Both tensor names are required by the current runtime. Image dimensions must be
multiples of 32. A successful ONNX export or TensorRT compilation alone does not
verify output names, class mappings, or detection quality.

## Exporter limitation

The generic training/export examples in [README.md](README.md) are experiments;
they do not reproduce this deployed artifact. The checked-in Hydra exporter
indexes heads by task type, so two object-detection heads collapse into one entry.
Its generated tensor names are `object_output`, `pose_output`, and segmentation
outputs, which do not satisfy the detector's two named object-output contract.

Use an artifact with the required contract. Producing a compatible replacement
requires the matching export implementation and trained checkpoints. Do not
change `detection.model_name` to a generic example export without verifying its
inputs, outputs, and label mappings against the detector.

## Compile and deploy

Run from the repository root against a provisioned K1. The runtime container must
be running and contain compatible ONNX Runtime and TensorRT libraries.

```sh
MODEL='yolo26m~hslvision=f17+yolo26m~hslvision+yolo26m~hslvision~jail.onnx'
git lfs pull
./pepsi hulk stop 42
./pepsi tensor-rt-compile "etc/neural_networks/$MODEL" 42 -- --raw_bytes_input 272,320,6
./pepsi upload 42
```

The shape above is for a 640×544 NV12 image; select the dimensions used by the
actual camera pipeline. The compiler upload uses clean synchronization, so stop
HULK first and restore the application with the normal upload afterward.
Keep the model, detection parameters, and target-compatible TensorRT cache together.

See [TensorRT compilation](../../tensorrt-compile/README.md) for prerequisites,
manual compilation, upload behavior, and failure handling. Verify startup,
model outputs, and detection quality on the target robot after deployment.
