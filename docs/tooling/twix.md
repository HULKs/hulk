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

## Panels and keybindings

ROS-Z Twix currently contains Text, Image, Map, Parameter, and Audio panels.
The Text panel observes one ROS-Z topic through `ros-z-debug` and renders the latest dynamic payload as JSON.
The Image panel observes `TimeWrapper<ros2::sensor_msgs::image::Image>` topics, defaults to `inputs/left_image`, and renders the latest raw camera frame.
The Parameter panel discovers ROS-Z nodes with remote parameter services, shows full snapshots or selected paths as JSON, and writes selected paths to active layers with revision checks.
The Audio panel defaults to `audio_spectrums` and displays microphone spectra and a waterfall for the selected channel.

The Text panel's **Topic** input accepts both a topic and a nested field path. Enter just the topic to display its whole message, or append a field path with a dot and press Enter:

- `detected_objects.inner` selects the wrapped detections.
- `detected_objects.inner[2].bounding_box.confidence` selects the third detected object's confidence. Arrays and sequences use zero-based indices, and paths can continue through nested structs and arrays.
- `status.state::Walking.speed` selects a field in an enum variant's payload. The selected value is unavailable while another variant is active.
- `topic."field.with.dots"` selects a field whose name contains punctuation. Field and variant names can be JSON-quoted.

Topics and fields autocomplete in the same input. Array completions include templates such as `detected_objects.inner[...]`. Selecting a template highlights `...` so you can replace it with an index, move past the closing bracket, and continue into the element's fields. Templates also work when the current array is empty. For a topic whose root is an array, use `topic[2]`.

Press **Ctrl+F** to focus and select the topic input in the active Text or Image panel. Press **Ctrl+Space** in a completion input to open its dropdown without changing the text, including when the input is empty. Use the arrow keys to choose a completion and Enter to apply it.

Present optionals are unwrapped when continuing through a path. Absent optionals and missing sequence elements appear as unavailable values; invalid paths show an error. Selecting an optional or enum itself displays its full value. Map entries are not traversable in this first version, but the whole map can be displayed.

Field completions use the schema from the latest received sample of the selected topic, including fields in absent optionals and inactive enum variants. Select a topic first to browse its fields, or enter a full path directly. Sequence index suggestions use the current length and show at most 64 indices; larger indices can be entered manually. Changing the field does not reconnect the topic subscription. Existing saved topic/field selections restore into the combined input. Displayed timestamps and publication metadata still refer to the original message. For topic names containing dots, the longest matching discovered topic takes precedence when separating topic and field path.

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

Supported action names are `open_split`, `open_tab`, `focus_namespace`, `focus_panel`, `focus_topic`, `focus_left`, `focus_below`, `focus_above`, `focus_right`, `close_tab`, `duplicate_tab`, `close_all`, and `no_op`. The `focus_topic` binding is configurable in `hulks/twix-ros-z.toml`, like the other actions.

Directional focus selects the nearest visible panel and outlines it.
Press `Tab` after moving focus to enter that panel's controls; subsequent `Tab` and `Shift-Tab` presses follow the normal control order.
