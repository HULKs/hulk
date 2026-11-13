# Localization and coordinate frames

## From measurements to a field pose

`localization-3d` uses field-mark associations, IMU data, visual odometry, and kinematic information to estimate the robot's pose.
The backend uses a factor graph. The node publishes a timestamped 3D estimate on `localization/pose_3d`.  
[3D localization](https://github.com/HULKs/hulk/blob/main/crates/nodes/localization-3d/src/node.rs) · [Localization backend](https://github.com/HULKs/hulk/blob/main/crates/localization-factrs/src/backend.rs) · [Vision](vision.md)

`localization-2d` matches each 3D estimate to the nearest timestamped `robot_to_ground` transform within its accepted time window.
When both are available, it publishes `ground_to_field` as `Isometry2<Ground, Field>`, using the source time of the 3D pose.
If either input is unavailable for that timestamp, the node skips that update.  
[2D localization](https://github.com/HULKs/hulk/blob/main/crates/nodes/localization-2d/src/lib.rs)

## Coordinate frames

| Frame | Meaning | Typical use |
| --- | --- | --- |
| `Ground` | Robot-relative plane with the origin between the soles, projected onto the ground. Positive x points forward. | Local ball, obstacles, walking, and kick direction. |
| `Field` | Field-centered frame with positive x pointing toward the opponent's goal. | Tactics, teammate poses, and shared targets. |
| `World` | Field-centered frame with side-independent orientation. | Geometry that stays fixed when the team-side convention changes. |
| `NormalizedField` | Field coordinates scaled by half the field length and width. | Configured positions such as kickoff poses. |

[Frame definitions](https://github.com/HULKs/hulk/blob/main/crates/coordinate_systems/src/lib.rs) · [World-to-field transform](https://github.com/HULKs/hulk/blob/main/crates/nodes/world_to_field_provider/src/lib.rs)

Multiplying a point in `Ground` by `ground_to_field` expresses it in `Field`.
The inverse expresses a `Field` target in the receiving robot's `Ground` frame.
Each robot uses its own localization estimate, so observations expressed in `Field` can still differ across teammates.  
[Kicking](../motion/kicking.md#behavior-inputs) · [Path planning](../motion/path_planning.md) · [Team communication](hsl_network.md)
