# Vision

## Image acquisition and detection

`image_receiver` publishes left and right images, camera calibration information, and frame-matched stereo pairs.
The detector subscribes to `inputs/left_image` and publishes timestamped detections on `detected_objects`.
The configured ONNX model supplies detections of balls, robots, goalposts, and field marks.
The camera matrix provides the projection between the image and robot-relative geometry.  
[Image receiver](https://github.com/HULKs/hulk/blob/main/crates/nodes/image_receiver/src/lib.rs) · [Detector](https://github.com/HULKs/hulk/blob/main/crates/nodes/detection/src/lib.rs) · [Detection parameters](https://github.com/HULKs/hulk/blob/main/etc/parameters/base/detection.json5) · [Camera matrix calculator](https://github.com/HULKs/hulk/blob/main/crates/nodes/camera_matrix_calculator/src/lib.rs)

## Consumers

| Consumer | Use of vision data |
| --- | --- |
| `ball_filter` | Projects ball candidates into `Ground` and updates local ball hypotheses. See [ball tracking](filters.md#local-ball). |
| `obstacle_filter` | Projects detections and tracks obstacles. It can also use teammate poses. See [obstacle tracking](filters.md#obstacles). |
| `field_mark_association` | Associates field marks with the field model for [localization](localization.md). |
| `stereo_visual_odometry` | Estimates camera motion from stereo image pairs. |

[Field-mark association](https://github.com/HULKs/hulk/blob/main/crates/nodes/field_mark_association/src/node.rs) · [Stereo visual odometry](https://github.com/HULKs/hulk/blob/main/crates/nodes/stereo_visual_odometry/src/lib.rs)

## Inspection

In Twix, select `inputs/left_image` and enable the object detection overlay to inspect bounding boxes, class labels, and confidence values.
[Twix image overlays](../../tooling/twix.md#image-overlays)

The former NAOv6 segmentation, line detection, and foot detection pipeline is described in the [historical vision page](https://github.com/HULKs/hulk/blob/37f26bf0f9d9eb87294cd874db395cc21a52c19d/docs/robotics/perception/vision.md).
