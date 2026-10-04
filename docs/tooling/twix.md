# Twix

Twix is the ROS-Z debugging UI.

Run it from the repository with:

```bash
./twix /42
```

The positional namespace is optional. If provided, it must be an absolute ROS-Z namespace such as `/42`; bare values such as `42` are invalid. If omitted, Twix uses the last stored namespace and then falls back to `/`.

To connect through a specific Zenoh router endpoint at startup, pass `--router`:

```bash
./twix /42 --router tcp/127.0.0.1:7447
```

To start with one blank workspace instead of restoring the saved session, run Twix with `--clear`. User presets are retained.

Twix checks the local repository version at startup and warns when the running binary is older than the checked-out `tools/twix/Cargo.toml` version. Use `--repository-root <path>` to point that check at a different checkout.

## Workspaces and presets

Workspaces are named groups in one shared docking layout. Every tab bar uses the same controls and drag-and-drop behavior.
Drag a panel to the outer tab bar, drag an entire workspace into another tab group, or drop beside a group to split the layout.
Moving panels preserves their live state and unfinished edits. Hover over a tab while dragging to reveal its contents.

Click **+** in any tab bar to search panel types, bundled layouts, and user presets.
The outer tab bar also offers **Blank workspace**. Search is focused immediately;
use the arrow keys and Enter to open a layout, or Escape to cancel without creating a tab.
Click a tab to switch, or right-click its title to rename it.
The close icon or middle-click closes a panel or an entire group immediately, discarding its current state.
Closing the final panel leaves a fresh Text panel.

The **+** picker contains bundled layouts (**Overview**, **Vision**, **Parameters**) and your saved layouts.
Opening a preset inserts a fresh copy into the chosen tab group, leaving existing panels intact.
Changes affect only that working copy. Right-click any panel or group and choose **Save as preset…** to save that subtree;
an existing name changes the action to **Overwrite**. The name editor is focused immediately and stays open while editing.
The trash icon beside a user preset in the **+** picker deletes it after confirmation; open copies are unaffected.
The save dialog selects its name field immediately; type a name and press Enter to save, or Escape to cancel.

Presets include nested groups, names, splits, active tabs, focus, and panel settings. Namespace, router, and theme remain global.
The whole docking layout is autosaved and restored on restart. Named groups retain their identity when moved or reduced to one child.
Only saved panel settings are restored, not live samples or unfinished parameter value edits.
Sessions and presets use the same saved layout format.
An unreadable session opens a blank workspace and displays an error; the original data is preserved
under `workspace_session_recovery` in eframe storage when the new session is saved.

User presets are JSON files in `dirs::config_dir()/hulks/twix-ros-z/presets/`
(`$XDG_CONFIG_HOME/hulks/twix-ros-z/presets/` on Linux, defaulting to `~/.config/hulks/twix-ros-z/presets/`).
To contribute a bundled preset, save it through the UI, copy the file to `tools/twix/presets/`,
and register it in `tools/twix/src/presets.rs`. Bundled files are embedded in the binary.
Validation checks the tree structure before constructing any panels.
Run `cargo test -p twix --bin twix` for preset validation, session, and headless layout tests.
These cover bundled presets, file operations, picker dialogs, and layout rendering and round trips.

## Panels and keybindings

Twix includes Audio, Text, Image, Map (2D), 3D Map, Parameter, and Timeline panels. Panels follow the selected namespace. Topic views use the shared `ros-z-debug` topic observer. The Text panel renders the latest dynamic payload as JSON. The Parameter panel discovers ROS-Z nodes with remote parameter services, shows full snapshots or selected paths as JSON, and writes selected paths to active layers with revision checks.

## MCAP Timeline

Start a recording with `cargo run -p mcap-replay -- recovered.mcap`. The tool starts
an embedded Zenoh router and prints its router URI and a Twix connection command.
Use `--router <URI>` to connect the replay tool to an existing router instead, or
`--listen tcp/127.0.0.1:0` to allocate an available embedded-router port.

In Twix, open **+ → Timeline**. Dock it alongside other views or save it in a
layout. Its Blender-style view uses the full panel, with an adaptive time ruler,
shaded recording range, recording strip, and a blue playhead extending through
the grid. The panel header holds play/pause, start/end, and step buttons.
Hover buttons for their labels; **⋯** holds exact-time jumps, step size, and
**Copy timestamp**. The step size is saved with the panel. Scrubbing follows input
immediately and sends seeks without waiting for status polling; intermediate
unsent positions are coalesced. Seeking pauses playback and resets histories and presentation
anchors, including on backward seeks. Multiple Timeline panels share a single
control connection; hiding or closing a Timeline does not interrupt synchronization
of the other views. Connection failures offer a retry action.

Click or drag anywhere in the grid to scrub. Wheel/pinch zooms around the pointer;
middle-drag or Shift + wheel pans without seeking. **Frame all** or a double
middle-click restores the full recording. With the timeline focused, **Space**
toggles playback, **Left/Right** step (hold Shift for finer steps), **Home/End**
jump to the bounds, and **F** frames all. Time labels are elapsed recording time,
not frame numbers. Each Timeline panel keeps an independent zoom/pan view.

See [MCAP replay](../../tools/mcap-replay/README.md) for topic semantics and the
Zenoh control protocol.

## 3D Map

Select **3D Map** in the panel selector. Its **Layers** menu independently toggles the field, articulated K1 robot, camera frustum/image plane, and field-mark associations. Drag with the left mouse button to orbit, drag with the right button to pan, and scroll to zoom. Layer settings and the selected pose source are saved with the layout.

Robot and camera placement can use localization or visual odometry. Visual odometry has its own coordinate frame, so it is not aligned with the displayed field; field-mark associations are only drawn with localization selected. Localization uses the latest pose. Camera matrices and kinematics are matched to the displayed frame within 100 ms using payload timestamps. Associations require the current localization epoch and, when images are available, an exact image timestamp. Frame timestamps never move backward within a namespace and epoch. Without camera or association frames, the robot uses the latest kinematics.

K1 meshes are compiled into Twix; no mesh files or source checkout are needed at runtime. The panel requires a WGPU-capable graphics adapter.

## Image Overlays

The Image panel defaults to `inputs/left_image`. It presents the newest complete image and enabled detection results together, using their exact acquisition timestamps. Empty detection results count as complete. Frames advance monotonically, skipping intermediate complete frames. If inputs are missing, the previous complete image and overlays remain visible unchanged: no timeout, partial-frame fallback, or waiting message replaces them. Image-topic, namespace, and publisher changes clear retained presentation state.

Histories are bounded by record count rather than age, so delayed transport does not automatically reject matching data. **Image history samples** controls the persisted image capacity (256 by default); each overlay observation retains up to 4,096 records. Two seconds of differential delay at 60 camera frames per second requires more than 120 image records plus margin. Increase image capacity for larger delays or rates, accounting for image memory usage. Matching requires all inputs to remain within their respective capacities; the displayed snapshot stays pinned even after history eviction.

Enable **Overlays > Projected Field Lines** to project the field using `localization/pose_3d` and camera transforms evaluated at the displayed image time. Geometry requires valid bracketing samples, compatible localization epochs and camera calibration, and never extrapolates. Invalid localization or a localization epoch change immediately removes the old projection. Explicitly unavailable localization permits new image/detection frames without a field projection; missing geometry remains pending. Valid calibrated intrinsics override camera intrinsics as configuration captured with the frame, not as a continuously changing paint input. Field associations never control image timing; matching-epoch, exact-frame associations add optional magenta residuals, including when they arrive after the image is displayed. Curves use at most 256 segments and skip pieces behind the camera.

Use the object-detection overlay for bounding boxes, class labels, and confidence values, and pose detection for human skeletons. Overlays are supported for the selected namespace's `inputs/left_image`, their producer's camera. Untimestamped legacy line/ball debug pixels and field-border candidate points are omitted because they cannot be aligned safely; the overlay menu explains unavailable layers. Timestamped field-border lines remain available. Zero image timestamps are reported as unsupported. Image and 3D Map panels select their frames independently.

## Keybindings

ROS-Z Twix reads keybindings from `hulks/twix-ros-z.toml`. Legacy Twix keeps using `hulks/twix.toml`, so the two tools do not share incompatible keybinding schemas. The default ROS-Z keybindings are:

| Key | Action |
| --- | --- |
| `C-t` | `open_split` |
| `C-T` | `open_tab` |
| `C-o` | `focus_namespace` |
| `C-p` | `focus_panel` |
| `C-h`, `C-Left` | `focus_left` |
| `C-j`, `C-Down` | `focus_below` |
| `C-k`, `C-Up` | `focus_above` |
| `C-l`, `C-Right` | `focus_right` |
| `C-w` | `close_tab` |
| `C-d` | `duplicate_tab` |
| `C-S-Backspace` | `close_all` |

Supported action names are `open_split`, `open_tab`, `focus_namespace`, `focus_panel`, `focus_left`, `focus_below`, `focus_above`, `focus_right`, `close_tab`, `duplicate_tab`, `close_all`, and `no_op`.

Directional focus selects the nearest visible panel and outlines it. Press `Tab` after moving focus to enter that panel's controls; subsequent `Tab` and `Shift-Tab` presses follow the normal control order.
