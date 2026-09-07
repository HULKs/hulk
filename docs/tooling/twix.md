# Twix

Twix is the ROS-Z debugging UI.

Run it from the repository with:

```bash
./twix /42
```

The positional namespace is optional. If provided, it must be an absolute ROS-Z
namespace such as `/42`; bare values such as `42` are invalid. If omitted, Twix
uses the last stored namespace and then falls back to `/`.

To connect through a specific Zenoh router endpoint at startup, pass `--router`:

```bash
./twix /42 --router tcp/127.0.0.1:7447
```

To start with one blank workspace instead of restoring the saved session, run
Twix with `--clear`. User presets are retained.

Twix checks the local repository version at startup and warns when the running
binary is older than the checked-out `tools/twix/Cargo.toml` version. Use
`--repository-root <path>` to point that check at a different checkout.

## Workspaces and presets

Workspaces are named groups in one shared docking layout. Every tab bar uses the
same controls and drag-and-drop behavior. Drag a panel to the outer tab bar,
drag an entire workspace into another tab group, or drop beside a group to split
the layout. Moving panels preserves their live state and unfinished edits. Hover
over a tab while dragging to reveal its contents.

Click **+** in any tab bar to search panel types, bundled layouts, and user
presets. The outer tab bar also offers **Blank workspace**. Search is focused
immediately; use the arrow keys and Enter to open a layout, or Escape to cancel
without creating a tab. Click a tab to switch, or right-click its title to
rename it. The close icon or middle-click closes a panel or an entire group
immediately, discarding its current state. Closing the final panel leaves a
fresh Text panel.

The **+** picker contains bundled layouts (**Overview**, **Vision**,
**Parameters**) and your saved layouts. Opening a preset inserts a fresh copy
into the chosen tab group, leaving existing panels intact. Changes affect only
that working copy. Right-click any panel or group and choose **Save as preset…**
to save that subtree; an existing name changes the action to **Overwrite**. The
name editor is focused immediately and stays open while editing. The trash icon
beside a user preset in the **+** picker deletes it after confirmation; open
copies are unaffected. The save dialog selects its name field immediately; type
a name and press Enter to save, or Escape to cancel.

Presets include nested groups, names, splits, active tabs, focus, and panel
settings. Namespace, router, and theme remain global. The whole docking layout
is autosaved and restored on restart. Named groups retain their identity when
moved or reduced to one child. Only saved panel settings are restored, not live
samples or unfinished parameter value edits. Sessions and presets use the same
saved layout format. An unreadable session opens a blank workspace and displays
an error; the original data is preserved under `workspace_session_recovery` in
eframe storage when the new session is saved.

User presets are JSON files in `dirs::config_dir()/hulks/twix-ros-z/presets/`
(`$XDG_CONFIG_HOME/hulks/twix-ros-z/presets/` on Linux, defaulting to
`~/.config/hulks/twix-ros-z/presets/`). To contribute a bundled preset, save it
through the UI, copy the file to `tools/twix/presets/`, and register it in
`tools/twix/src/presets.rs`. Bundled files are embedded in the binary.
Validation checks the tree structure before constructing any panels. Run
`cargo test -p twix --bin twix` for preset validation, session, and headless
layout tests. These cover bundled presets, file operations, picker dialogs, and
layout rendering and round trips.

## Panels and keybindings

ROS-Z Twix currently contains Text, Image, Map, and Parameter panels. The Text
panel observes one ROS-Z topic through `ros-z-debug`, renders the latest dynamic
payload as JSON, and shows sample metadata. The Image panel observes
`TimeWrapper<ros2::sensor_msgs::image::Image>` topics, defaults to
`inputs/left_image`, and renders the latest raw camera frame. The Parameter
panel discovers ROS-Z nodes with remote parameter services, shows full snapshots
or selected paths as JSON, and writes selected paths to active layers with
revision checks.

The Image panel's Overlays menu offers confidence controls for object and pose
detections. Bounding boxes default to `0.5` confidence and pose keypoints to
`0.8`, including when an older layout has no saved thresholds. Values range from
`0` to `1` and are saved with the panel layout. These controls only filter the
visualization; they do not change the detector or its published results.

ROS-Z Twix reads keybindings from `hulks/twix-ros-z.toml`. Legacy Twix keeps
using `hulks/twix.toml`, so the two tools do not share incompatible keybinding
schemas. The default ROS-Z keybindings are:

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

Supported action names are `open_split`, `open_tab`, `focus_namespace`,
`focus_panel`, `focus_left`, `focus_below`, `focus_above`, `focus_right`,
`close_tab`, `duplicate_tab`, `close_all`, and `no_op`.

Directional focus selects the nearest visible panel and outlines it. Press `Tab`
after moving focus to enter that panel's controls; subsequent `Tab` and
`Shift-Tab` presses follow the normal control order.
