# Twix

Twix is the ROS-Z debugging UI.

Run it from the repository with:

```bash
./twix /42
```

The positional namespace is optional.
If provided, it must be an absolute ROS-Z namespace such as `/42`; bare values such as `42` are invalid.
If omitted, Twix uses the last stored namespace and then falls back to `/`.

To connect through a specific Zenoh router endpoint at startup, pass `--router`:

```bash
./twix /42 --router tcp/127.0.0.1:7447
```

To start with one blank workspace instead of restoring the saved session, run Twix with `--clear`.
User presets are retained.

Twix checks the local repository version at startup and warns when the running binary is older than the checked-out `tools/twix/Cargo.toml` version.
Use `--repository-root <path>` to point that check at a different checkout.

## Workspaces and presets

Workspaces are named groups in one shared docking layout.
Every tab bar uses the same controls and drag-and-drop behavior.
Drag a panel to the outer tab bar, drag an entire workspace into another tab group, or drop beside a group to split the layout.
Moving panels preserves their live state and unfinished edits.
Hover over a tab while dragging to reveal its contents.

Click **+** in any tab bar to search panel types, bundled layouts, and user presets.
The outer tab bar also offers **Blank workspace**.
Search is focused immediately; use the arrow keys and Enter to open a layout, or Escape to cancel without creating a tab.
Click a tab to switch, or right-click its title to rename it.
The close icon or middle-click closes a panel or an entire group immediately, discarding its current state.
Closing the final panel leaves a fresh Text panel.

The **+** picker contains bundled layouts (**Overview**, **Vision**, **Parameters**) and your saved layouts.
Opening a preset inserts a fresh copy into the chosen tab group, leaving existing panels intact.
Changes affect only that working copy.
Right-click any panel or group and choose **Save as preset…** to save that subtree; an existing name changes the action to **Overwrite**.
The name editor is focused immediately and stays open while editing.
The trash icon beside a user preset in the **+** picker deletes it after confirmation; open copies are unaffected.
The save dialog selects its name field immediately; type a name and press Enter to save, or Escape to cancel.

Presets include nested groups, names, splits, active tabs, focus, and panel settings.
Namespace, router, and theme remain global.
The whole docking layout is autosaved and restored on restart.
Named groups retain their identity when moved or reduced to one child.
Only saved panel settings are restored, not live samples or unfinished parameter value edits.
Sessions and presets use the same saved layout format.
An unreadable session opens a blank workspace and displays an error; the original data is preserved under `workspace_session_recovery` in eframe storage when the new session is saved.

User presets are JSON files in `dirs::config_dir()/hulks/twix-ros-z/presets/` (`$XDG_CONFIG_HOME/hulks/twix-ros-z/presets/` on Linux, defaulting to `~/.config/hulks/twix-ros-z/presets/`).
To contribute a bundled preset, save it through the UI, copy the file to `tools/twix/presets/`, and register it in `tools/twix/src/presets.rs`.
Bundled files are embedded in the binary.
Validation checks the tree structure before constructing any panels.
Run `cargo test -p twix --bin twix` for preset validation, session, and headless layout tests.
These cover bundled presets, file operations, picker dialogs, and layout rendering and round trips.

## Panels

Choose a panel from the **+** picker.

| Panel | Purpose |
| --- | --- |
| Text | Display a topic's latest payload or a selected field as JSON. |
| Image | Display the latest raw camera frame from a `ros2::sensor_msgs::image::Image` topic, defaulting to `inputs/left_image`. |
| Map | Display field or ground views with selectable layers for robot pose, balls, obstacles, and paths. |
| Parameter | Discover nodes with remote parameter services, display snapshots or selected paths as JSON, and write selected paths to active layers with revision checks. |
| Audio | Display microphone spectra and a waterfall for the selected channel, defaulting to `audio_spectrums`. |
| [Plot](#plot-panel) | Compare numeric topics or fields over time, with enum and boolean states in the background and parameters as threshold lines. |
| [Timeline](#mcap-timeline) | Seek and play back an MCAP recording. |

## Topic and field selection

The Text and Plot panels' **Topic** inputs accept both a topic and a nested field path.
In Text, enter just the topic to display its whole message, or append a field path with a dot and press Enter:

- `detected_objects.inner` selects the wrapped detections.
- `detected_objects.inner[2].bounding_box.confidence` selects the third detected object's confidence.
  Arrays and sequences use zero-based indices, and paths can continue through nested structs and arrays.
- `status.state::Walking.speed` selects a field in an enum variant's payload.
  The selected value is unavailable while another variant is active.
- `topic."field.with.dots"` selects a field whose name contains punctuation.
  Field and variant names can be JSON-quoted.

Topics and fields autocomplete in the same input.
Array completions include templates such as `detected_objects.inner[...]`.
Selecting a template highlights `...` so you can replace it with an index, move past the closing bracket, and continue into the element's fields.
Templates also work when the current array is empty.
For a topic whose root is an array, use `topic[2]`.

Press **Ctrl+F** to focus and select the topic input in the active Text, Image, Audio, or live Plot panel.
In Plot, this focuses the first line; adding a line focuses its own input.
Press **Ctrl+Space** or **Arrow Down** in a completion input to open its dropdown, including when the input is empty.
Arrow Down also highlights the first suggestion.
Use the arrow keys to choose a completion and Enter to apply it.
The input keeps focus with the cursor after the completion, so you can continue typing the path.

Present optionals are unwrapped when continuing through a path.
Absent optionals and missing sequence elements appear as unavailable values; invalid paths show an error.
In Text, selecting an optional or enum itself displays its full value.
Map entries are not traversable in this first version, but Text can display the whole map.

Field completions use the schema from the latest received sample of the selected topic, including fields in absent optionals and inactive enum variants.
Select a topic first to browse its fields, or enter a full path directly.
Sequence index suggestions use the current length and show at most 64 indices; larger indices can be entered manually.
Changing the field does not reconnect the topic subscription.
Layouts save the topic and field path separately and restore them into the combined input.
The saved field path keeps its leading separator, for example `.inner[2]` or `[0].x`.
For topic names containing dots, the longest matching discovered topic takes precedence when separating topic and field path.

## Plot panel

Enter a numeric topic or field in the **Topic** input, for example `detected_objects.inner[2].bounding_box.confidence`.
Use **Add item** to compare several sources on the same axes, and hover over a line to see its values.
Each item's style button switches between a line and individual samples, and its info button shows the item's status and any selection problem.
An icon at the start of each row shows the item type: a line chart for numeric topic lines, a timeline for topic states, and a threshold icon for parameter thresholds.
Gaps in a line mean the value was unavailable, for example an absent optional or a missing array element.

Select an enum or boolean field, for example `primary_state`, to show its states as labeled, colored intervals behind the numeric lines.
This helps correlate state changes, such as transitions between motion states, with numeric data.
Each state item gets its own horizontal lane, so several enums can be compared at once; hiding an item through its checkbox gives its lane to the others.
Faint interval colors identify the variants and stay the same for a given enum, keeping the lines in the foreground.
Because variants have their own colors, the color picker is disabled for state items; the chosen color is kept for switching back to a numeric field.
Labels that do not fit an interval are shortened; hover over an interval away from the lines to see the item, state, and its time range.
States narrower than a pixel merge into one neutral band, so fast-changing values stay cheap to draw; zoom in while paused to tell them apart.
A state lasts until a sample with another variant arrives, and the current state extends to the newest displayed time.
Unavailable values, such as absent optionals or inactive parent variants, end the current state and count as gaps.
States follow the same time axis, history window, and pause/zoom behavior as the lines, and do not affect the Y axis range.

Use **Add threshold** to draw a parameter as a dashed horizontal line, for example while tuning a threshold that guards an output.
Enter the parameter **Node**, relative to the robot namespace or absolute, and a dot-separated **Path**, for example node `obstacle_filter` and path `robot_confidence_threshold`.
Like topics, both inputs apply when you press Enter or choose a completion.
Nodes complete from discovered parameter services, and paths complete from the node's parameter snapshot.
A number draws one line, and an array of numbers, such as `[0.05, 0.1]`, draws one line per element.
Thresholds follow the node's parameter events, so a line moves when the parameter changes, including writes from the Parameter panel.
Thresholds have the same color, visibility, info, and remove controls as topic items.
They are drawn without topic data and are included in the Y axis range.
If the node is unavailable, the info button shows the error, the last known value stays visible, and Twix keeps retrying.

The X axis shows the seconds before the newest sample, and **History** sets the time window.
**Pause**, or **Space** after clicking the plot, freezes it so you can zoom and pan with the mouse or touchpad.
Pausing also freezes threshold values until the plot resumes.
**Reset view** or a double-click restores the full time window.

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
**Copy timestamp**, which copies elapsed seconds as a decimal directly usable in **Jump to**. The popup stays open while editing; editing the jump time pauses playback. The step size is saved with the panel. Scrubbing follows input
immediately and updates a client-local playback cursor. Seeking pauses playback and
invalidates obsolete query results, including on backward seeks. Multiple Timeline
panels share one cursor; hiding or closing a Timeline does not stop playback.
Different Twix clients have independent playback. Connection failures offer a retry action.

Click or drag anywhere in the grid to scrub. Wheel/pinch zooms around the pointer;
middle-drag or Shift + wheel pans without seeking. **Frame all** or a double
middle-click restores the full recording. By default, with the timeline focused, **Space**
toggles playback (including after using the toolbar), **Left/Right** step (hold Shift for finer steps), **`,` / `.`** step backward/forward by exactly 2 ms, **Home/End**
jump to the bounds, and **F** frames all. Time labels are elapsed recording time,
not frame numbers. Each Timeline panel keeps an independent zoom/pan view.
These shortcuts are [rebindable](#keybindings); toolbar tooltips show the configured keys.

See [MCAP replay](https://github.com/HULKs/hulk/blob/main/tools/mcap-replay/README.md) for server setup and command-line options.

## Image Overlays

The Image panel defaults to `inputs/left_image`.
If that topic is absent, it automatically observes `inputs/stereo_image_pair` and extracts its left image.
The primary topic wins whenever it is present, regardless of whether data has arrived yet.
Live discovery updates reselect the source; unrelated topics do not recreate its observation.
The stereo fallback projects `.left.header.stamp` instead of `.header.stamp`.
In Live mode, enabled object and pose detections contribute their latest available acquisition timestamps. The minimum available timestamp selects an exact image from retained history; if that image is absent, the panel selects the latest image. There is no fixed delay. A stalled detector can keep an older image selected while that image remains retained.
The panel observes one query plan: select the exact preferred image or latest image, then attach independent overlay inputs. Live evaluates these operations against the same retained source views; there are no asynchronous per-frame lookups chasing the selected image. Only the selected image is decoded, and its texture is reused across unchanged results.
Replay selects the newest image at or before the timeline cursor, independently of overlay availability. The same plan attaches inputs against that image's acquisition time, including results published or logged later than the cursor. Partial responses can display the image while optional overlay indexes are still building.
Each overlay prepares and draws independently. Empty detections are valid data; missing, invalid, or failed inputs suppress only the affected overlay. Late results can populate a still-displayed frame, but cannot paint on another frame.
Image-topic, namespace, backend, or overlay-selection changes rebuild the plan observation. Compatible Live feeds remain shared, and overlay-only changes retain the image render cache.
The Horizon overlay interpolates compatible camera matrices over brackets spanning at most 100 ms.

Live ingestion is independent of the UI frame rate. Shared histories are bounded by sample count and bytes; observations report when retention limits their coverage. Replay uses server-side indexes instead of accumulating panel histories. Image and overlay timestamp projections are developer-owned query declarations; legacy saved projection overrides are ignored. The Text panel always uses source time, as do other observations unless explicitly configured by code.

The Image panel's Overlays menu offers confidence controls for object and pose detections.
Bounding boxes default to `0.5` confidence and pose keypoints to `0.8`.
Values range from `0` to `1` and reset to their defaults when the overlay is toggled off and on or Twix is reopened.
These controls only filter the visualization; they do not change the detector or its published results.

### Adding an image overlay

Implement `ImageOverlay` in `tools/twix/src/panels/image/overlays/` and register an instance in `ImageOverlays::new`. `inputs()` declares relative topics, projections, query selections, subscription policy, and optional frame preferences.
The image panel compiles these declarations into a single query plan. `prepare(time, &mut inputs)` clears old prepared state, decodes its already-matched slots through `OverlayInputs::decode` in `image/inputs.rs`, and computes drawing data.
Decoding retains diagnostics even when optional-input failures use fallback geometry; errors appear in the overlay menu.
`image_overlay()` draws that state when available.
Optional `dropdown_ui()` controls appear beneath the overlay in the Overlays menu.
The registry owns enablement, persistence, input ranges, and preparation caching; concrete overlays do not access the backend.
Shared camera/interpolation helpers live in `overlays/camera.rs`.
The texture cache retains sample identity and pixels, not successful or failed wire payloads. Clearing the image topic releases the plan observation.
Both Live and replay use the same overlay declarations and preparation code.

Use the object-detection overlay for bounding boxes, class labels, and confidence values, and pose detection for human skeletons.
Overlays are supported for the selected namespace's `inputs/left_image`, their producer's camera.

## Keybindings

ROS-Z Twix reads keybindings from `hulks/twix-ros-z.toml`.
Legacy Twix keeps using `hulks/twix.toml`, so the two tools do not share incompatible keybinding schemas.
The default ROS-Z keybindings are:

| Key | Action |
| --- | --- |
| `C-t` | `open_split` |
| `C-T` | `open_tab` |
| `C-o` | `focus_namespace` |
| `C-p` | `focus_panel` |
| `C-f` | `focus_topic` |
| `C-h`, `C-Left` | `focus_left` |
| `C-j`, `C-Down` | `focus_below` |
| `C-k`, `C-Up` | `focus_above` |
| `C-l`, `C-Right` | `focus_right` |
| `C-w` | `close_tab` |
| `C-d` | `duplicate_tab` |
| `C-S-Backspace` | `close_all` |

Supported action names are `open_split`, `open_tab`, `focus_namespace`, `focus_panel`, `focus_topic`, `focus_left`, `focus_below`, `focus_above`, `focus_right`, `close_tab`, `duplicate_tab`, `close_all`, and `no_op`.

The following timeline actions are also supported. They apply only when the timeline or its toolbar has focus, not while editing a popup or another panel.

| Default key | Action |
| --- | --- |
| `Space` | `timeline_toggle_playback` |
| `ArrowLeft` | `timeline_step_backward` |
| `ArrowRight` | `timeline_step_forward` |
| `S-ArrowLeft` | `timeline_fine_step_backward` |
| `S-ArrowRight` | `timeline_fine_step_forward` |
| `Comma` | `timeline_nudge_backward` (2 ms) |
| `Period` | `timeline_nudge_forward` (2 ms) |
| `Home` | `timeline_start` |
| `End` | `timeline_end` |
| `f` | `timeline_frame_all` |

User bindings merge with the defaults by key. To move playback from Space to P, disable the old binding:

```toml
[keys]
Space = "no_op"
p = "timeline_toggle_playback"
```

Directional focus selects the nearest visible panel and outlines it.
Press `Tab` after moving focus to enter that panel's controls; subsequent `Tab` and `Shift-Tab` presses follow the normal control order.
