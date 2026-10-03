# Filters

Perception produces measurements from individual sensor samples. A filter combines those measurements over time to maintain a useful estimate even when observations are noisy or temporarily missing.
See the [robotics overview](../overview.md) for the ROS-Z node and topic model.

## Ball Filter: General Idea

The ball filter estimates the ball's position and velocity from camera detections and robot odometry.
It keeps several **hypotheses**, or possible ball tracks, rather than trusting every detection immediately.
Repeated matching observations strengthen a track; missing observations and implausible measurements weaken it.
The highest-validity hypothesis above the output threshold becomes the reported ball. If none qualifies, the output is `None`.

Each hypothesis contains:

- A **resting** model with position `(x, y)`, or a **moving** model with position and velocity `(x, y, vx, vy)`.
- A covariance matrix describing uncertainty in the estimate.
- A validity score expressing how well observations have supported the track. This score is not a probability.
- A `last_seen` timestamp recording the last observation that supported it.

Positions use the `Ground` coordinate system, relative to the robot. If the robot walks forward toward a stationary ball, the ball becomes closer in these coordinates.
Odometry lets the filter account for this change without mistaking robot motion for ball motion.

### Predict, Match, Update

1. **Predict:** on odometry events, advance hypotheses using elapsed time and odometry. Moving hypotheses extrapolate velocity, apply velocity decay, and add process uncertainty. Resting hypotheses still transform with robot motion.
2. **Project measurements:** select sufficiently confident ball detections and use the camera matrix to project their image positions onto the ground at ball-center height. Projected measurement uncertainty is carried with each percept.
3. **Match:** build a matrix of negative squared Mahalanobis distances and use a reusable `linear_sum_assignment::AssignmentSolver` with `Objective::Maximize` to choose global matches. Matches beyond `maximum_matching_cost` are rejected and penalize the track's validity; their percepts remain available to spawn new tracks.
4. **Update:** apply Kalman measurement updates to matched tracks, increase validity, and set `last_seen`. Unmatched percepts can start new tracks.
5. **Clean up:** discard timed-out, low-validity, or spatially out-of-bounds hypotheses, merge nearby resting tracks, and limit the number retained.

The spatial bound is currently a rectangle centered on the robot in `Ground`:
`abs(x) < field_dimensions.length / 2` and `abs(y) < field_dimensions.width / 2`.
It does not transform a hypothesis into `Field` to test the actual field boundary.
Velocity decay and process noise are applied per prediction, so their tuning
depends on the odometry cadence; detections alone do not run prediction.

Validity decays when processing usable detection frames. The decay factor depends on whether a hypothesis projects into the camera's view: failure to observe a ball that should be visible is different from failure to observe one outside the image.

## Inputs

| Topic | Consumption | Purpose |
| --- | --- | --- |
| `inputs/odometry` | Announced stream in a future map | Predict motion and transform robot-relative tracks. |
| `detected_objects` | Announced stream in the same future map | Supply image detections for measurement updates. |
| `camera_matrix` | Timestamp-indexed cache | Select camera geometry near the detection timestamp. |
| `field_dimensions` | Latest-value cache with transient-local durability | Supply ball radius and field dimensions. |

Transient-local durability allows a late subscriber to obtain retained field dimensions.
The loop skips batches until field dimensions are available. Missing camera
geometry prevents a detection update, while available odometry can still drive
prediction. The nearest camera matrix may precede or follow the detection.
Main has no maximum camera-time-difference gate: an available nearest matrix
is used regardless of its age, both for detection projection and image overlays.

## The Processing Loop

The entry point and processing helpers are in
`crates/nodes/ball_filter/src/lib.rs`.
After setting up parameters, subscriptions, and publishers, it repeats:

1. Take a parameter snapshot and await `future_map.recv()`.
2. Read the latest field dimensions.
3. Process the batch's persistent events in timestamp order, applying odometry prediction and detection updates where those inputs exist.
4. Clean up at the latest persistent timestamp, then select the best hypothesis.
5. Build ground-plane outputs and project sufficiently valid tracks back into the image for visualization.
6. Publish the results and retain the filter state for the next iteration.

The synchronous processing section uses `block_in_place`. This lets Tokio hand off other runtime work while the current task performs projection, matching, and matrix operations.
It does not run the iterations in parallel; the state updates finish before the output publication awaits.

### Why `output_time` Matters

`output_time` is the latest timestamp in the persistent batch. It represents the processed input timeline, not the time publication finishes.
The filter uses it to evaluate hypothesis timeouts. Main does **not** use it
to stamp the published ball position: all six outputs use ordinary `publish`.

For example, if a batch describes inputs through time `10.000 s` and publication
happens at `10.040 s`, main stamps the publication with the node clock at
publication, not `10.000 s`. Consumers aligning sensor truth must account for
this difference.
The ball's `last_seen` is a separate timestamp: the filter can predict a track forward without seeing it again.

When there are no persistent events, there is no `output_time`, so cleanup is
skipped, but the loop still publishes the retained estimate and other outputs.
For image visualization, `projection_time` can fall back to the earliest temporary timestamp to select camera geometry; temporary events are not applied to the long-lived filter state.

All main ball-filter outputs use publication time as their ROS-Z source
timestamp. The estimate's payload `last_seen` is separately updated on matched
observations. There is no schedule diagnostic in main.
The visual-kick selector consumes `ball_filter/ball_percepts` and assigns its
held percept's `last_seen` from that topic's publication source time, not the
original image time.

ROS-Z attachment source time is also distinct from a cache's index:
default caches use the Zenoh transport timestamp, while `.with_stamp(...)`
extracts time from the payload. Publishing with an explicit source time does
not automatically change default cache indexing. The ball filter's camera
cache explicitly extracts `TimeWrapper.time`.

## Outputs and Debugging

The node explicitly publishes these topics:

- `ball_filter/ball_position`: the best sufficiently valid ball position, velocity, and `last_seen`, or `None`.
- `ball_filter/ball_filter_state`: all hypotheses, including uncertainty and validity.
- `ball_filter/best_ball_hypothesis`: the selected full hypothesis, or `None`.
- `ball_filter/ball_percepts`: projected measurements collected in this batch.
- `ball_filter/filtered_balls_in_image`: image circles projected from sufficiently valid tracks.
- `ball_filter/hypothetical_ball_positions`: lower-validity candidates.

Use [Twix](../../tooling/twix.md) to inspect the state and image/map overlays.
To understand the algorithm, start with `lib.rs`, then `filter.rs` and
`hypothesis.rs` in `crates/nodes/ball_filter/src/`; the moving and resting
submodules implement the model-specific prediction and update steps.
