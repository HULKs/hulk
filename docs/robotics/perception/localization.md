# Localization and Coordinate Frames

Localization connects [vision](vision.md), robot geometry, and the Field-frame
state consumed by [behavior](../behavior/overview.md).
The current launcher starts field-mark association, stereo visual odometry,
3D localization, and the 2D localization adapter as separate ROS-Z nodes.

## Frames and Camera Geometry

- `Pixel` describes image coordinates.
- `Robot` describes the robot body frame.
- `Ground` is the robot-relative horizontal frame between its soles; it is used
  for ball tracks and local motion requests.
- `Field` describes the soccer field, allowing robot-relative observations and
  destinations to be expressed in a shared field frame.

`ground_provider` combines IMU orientation, robot kinematics, and support-foot
information to publish timestamped optional `robot_to_ground` and
`ground_to_robot` transforms. A missing support foot yields no usable transform.
`camera_matrix_calculator` combines `robot_to_ground`, nearby head kinematics,
and left-camera calibration from `inputs/camera_info` to publish `camera_matrix`.
Its wrapper time follows the ground-transform input, not publication completion.

## Localization Pipeline

1. **Field-mark association** consumes announced `detected_objects`, camera
   matrices, field dimensions, and `localization/association_pose_hint`.
   Supported landmark classes include goalposts, L/T/X spots, and penalty spots.
   It publishes timestamped `field_mark_association/visual_localization` frames,
   containing image-to-field associations and, when applicable, a backend pose reset.
2. **Stereo visual odometry** supplies relative camera-motion information and
   a current left-camera pose in its visual-odometer frame.
3. **3D localization** combines visual associations, visual odometry, IMU,
   and kinematic information. It publishes `localization/pose_3d` as a
   `TimeWrapper<Option<Isometry3<Field, Robot>>>` and supplies pose hints back to
   association. It waits for field dimensions and initial camera geometry during startup.
4. **2D localization** combines a valid 3D pose with a nearby valid
   `robot_to_ground` transform to publish `Isometry2<Ground, Field>` on
   `ground_to_field`. The maximum transform timestamp difference is 100 ms.
    The input wrapper time is used to select geometry, but the output currently uses ordinary `publish`, so its ROS-Z source timestamp is publication time rather than the localization wrapper time.

Association and 3D localization reset their tracking/localization state while
the primary state is Damping. The 2D adapter skips invalid poses or unavailable
geometry; it does not publish a separate invalidation message on `ground_to_field`.
Consumers using latest-value caches must therefore distinguish an available
cached transform from a newly updated pose.

## Time Alignment and Inspection

Sensor time, source metadata, and delivery time are separate. Camera and
localization caches that use `.with_stamp(...)` index the wrapper's sensor/state
time. Default caches index Zenoh transport time, even when a publisher supplies
an earlier ROS-Z attachment source time. See [Filters](filters.md#why-output_time-matters)
for the corresponding ball-estimation timestamp rules.

Useful Twix/recording topics include:

- `camera_matrix`, `robot_to_ground`, and `ground_to_field`.
- `field_mark_association/visual_localization` and `debug/global_localization`.
- `localization/pose_3d` and `localization/association_pose_hint`.
- `debug/calibrated_intrinsics` and `debug/solve_diagnostics`.
- `visual_odometry/current_left_camera_to_visual_odometer`.

Implementation entry points are `crates/nodes/ground_provider/src/lib.rs`,
`crates/nodes/camera_matrix_calculator/src/lib.rs`,
`crates/nodes/field_mark_association/src/node.rs`,
`crates/nodes/localization-3d/src/node.rs`, and
`crates/nodes/localization-2d/src/lib.rs`. Topic constants and visual-association
message types are in `crates/types/src/visual_localization.rs`.
