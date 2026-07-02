# Robot Viewer

Robot Viewer renders camera, perception, and robot-state streams using the displayed camera frame as
the temporal anchor.

## Temporal Alignment

- Camera images are buffered by their `TimeWrapper` timestamp.
- The displayed image timestamp is chosen from the latest field-mark association timestamp if that
  exact image is still buffered, then the latest detection timestamp if that exact image is still
  buffered, then the newest camera image.
- Association or detection frames more than 1 second older than the newest camera image are ignored
  as stale, so the viewer does not freeze on old processed output.
- Field-mark associations and detections are rendered only when their timestamp exactly matches the
  displayed image.
- Camera matrices and robot kinematics use the nearest sample within 100 ms of the displayed image.
- The renderer may reuse the last valid camera matrix or robot kinematics sample for up to 250 ms
  when a displayed frame temporarily lacks one, avoiding one-frame T-pose or projection flicker
  caused by message-ordering jitter.
- When `project field lines` is enabled, unique field-mark associations are shown as residual lines
  from the detected image feature to the current localization projection of its associated field
  point.
- Localization and visual-odometry poses are latest-value streams; the UI labels them as `latest`
  because their current topics do not carry a frame timestamp.

Object detections come from the announced `detected_objects` stream. Replays, simulators, and manual
test publishers must provide both `detected_objects` and matching `detected_objects/announce`
messages. If the announce stream is missing, the viewer intentionally shows detections as
unavailable rather than drawing boxes on the wrong image frame.

For replay or manual validation, publish a camera frame and matching announced detections with the
same timestamp, plus a camera matrix within 100 ms. The camera panel should show that timestamp in
the `aligned` footer, render the detection boxes, and report the camera-matrix time offset. Brief
matrix or kinematics gaps shorter than 250 ms should not make the overlay disappear or reset the
robot mesh to its fallback pose.
