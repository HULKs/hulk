# Vision

Vision runs as cooperating ROS-Z nodes, with messages connecting image acquisition, neural-network detection, camera geometry, and state estimation.
See the [robotics overview](../overview.md) for node startup and subscription mechanics.

## Images and Detection

On the robot, `image_receiver` obtains X5 camera data and publishes
`inputs/left_image`, `inputs/right_image`, camera calibration topics, and matched
`inputs/stereo_image_pair` messages. Detection uses the left image; stereo pairs
also support visual odometry.

The `detection` node in `crates/nodes/detection/src/lib.rs` subscribes to `inputs/left_image`.
It reads the image capture timestamp from the header and announces pending results on `detected_objects` before inference.
This allows downstream fusion nodes to track results that are still being computed.

The node runs a neural network through ONNX Runtime with TensorRT/CUDA execution providers, post-processes the outputs, and applies non-maximum suppression.
It then publishes timestamped object detections using the announced publication.
Model selection and detection thresholds are configured through the node's parameters.

### Input and Model Contract

Detection requires image encoding **`nv12`**, with width and height both divisible
by **32**. Unsupported encoding or dimensions return a node error.
The image bytes are passed to the model's `raw_bytes_input` tensor with shape
`[height / 2, width / 2, 6]`; a model that expects preprocessed RGB input is not a
drop-in replacement.

The model must provide both `hslvision_output` and `nao_output`, each with shape
`[1, 300, 6]`. These are the tensor names required by the current detector.
Each candidate contains bounding-box coordinates, confidence, and a class index,
as defined by `NUMBER_OF_VALUES_PER_OBJECT` in `crates/types/src/object_detection.rs`.
Detection checks both output shapes before extracting and combining candidates.
It does not publish a pose-detection topic.

`neural_networks_folder` and `model_name` select the model when the ONNX session
is built at node startup. Changing them requires restarting detection to load
another model. Enable, candidate-confidence, and intersection-over-union settings
are read during the loop and can affect subsequent images without rebuilding
the session. TensorRT engine caching is configured in the neural-network folder.

## Geometry and State Estimation

The `camera_matrix_calculator` supplies timestamped camera geometry for converting between image and ground coordinates.
The [ball filter](filters.md) combines detected balls with odometry, using a camera matrix near each detection's timestamp.
Field-mark association uses detected landmarks and timestamped camera geometry
to supply visual localization observations. Stereo visual odometry, IMU, and
kinematics feed 3D localization; a 2D adapter publishes the `ground_to_field`
transform used by behavior. See [Localization and coordinate frames](localization.md)
for this pipeline and its inspection topics.
Obstacle filtering consumes object detections and teammate state.
The source of images and the processing rate depend on the hardware and node configuration.

Use [Twix](../../tooling/twix.md) to inspect images, detection overlays, and filtered estimates.
The detection node also publishes `inference_duration`,
`post_processing_duration`, and `non_maximum_suppression_duration` for timing inspection.
