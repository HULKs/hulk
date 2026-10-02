# Robot Viewer

Robot Viewer renders camera, perception, and robot-state streams using the displayed camera frame as
the temporal anchor.

## Run

From the repository root, with a robot or simulator router already running:

```bash
cargo run -p robot_viewer -- --namespace /42 --router tcp/10.1.24.42:7447
```

`--robot 42` is shorthand for namespace `/42`; `--namespace` also accepts nested
namespaces. These options conflict. Omitting both uses `/`, and omitting `--router`
uses `tcp/localhost:7447`. Choose an endpoint reachable from the viewer; a robot's
loopback-only router cannot be reached from another host without forwarding or a
shared router. The desktop UI requires a display and suitable graphics support.

For the experimental [Alex simulator branch](https://github.com/alexschmander/hulk/tree/motion-inference-simulator), first check out and start its implementation, then use
`cargo run -p robot_viewer -- --namespace /simulator/robot --router tcp/127.0.0.1:7447`.
The simulator launcher and implementation are not available on main. The branch
simulator does not publish rendered camera images, so it can supply state topics
without providing the camera frames needed for image overlays.

## Temporal Alignment

- Camera images are buffered by their `TimeWrapper` timestamp.
- The displayed image timestamp is chosen from the latest field-mark association timestamp if that
  exact image is still buffered, then the latest detection timestamp if that exact image is still
  buffered, then the newest camera image.
- Association or detection frames more than 1 second older than the newest camera image are ignored
  when choosing a new aligned anchor. The display anchor is monotonic and can wait for detections
  rather than immediately selecting the newest image.
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

**Stale-publisher limitation:** after detections have arrived, a detection publisher
that remains registered but stops producing samples can leave the previous image
anchor pinned. The one-second candidate cutoff does not guarantee recovery from
this case. Check publisher state and sample arrival separately; reconnect/restart
the viewer after correcting the publisher if the display remains pinned.

For replay or manual validation, publish a camera frame and matching announced detections with the
same timestamp, plus a camera matrix within 100 ms. The camera panel should show that timestamp in
the `aligned` footer, render the detection boxes, and report the camera-matrix time offset. Brief
matrix or kinematics gaps shorter than 250 ms should not make the overlay disappear or reset the
robot mesh to its fallback pose.
