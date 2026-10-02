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

### Preset JSON format

Sessions and preset files use an unversioned JSON envelope with `tree`, nullable
`focused` (a tile ID), and optional `names` (tile IDs mapped to titles).
`tree` is the serialized `egui_tiles` tree: pane tiles contain `{ "kind": ..., "state": ... }`,
and container tiles describe tabs, linear splits or grids. For example, a minimal Text layout is:

```json
{
  "tree": {
    "id": 1,
    "root": 2,
    "tiles": {
      "next_tile_id": 3,
      "tiles": {
        "1": { "Pane": { "kind": "text", "state": { "topic": "behavior/motion_command", "pretty": true } } },
        "2": { "Container": { "Tabs": { "children": [1], "active": 1 } } }
      },
      "invisible": []
    },
    "height": null,
    "width": null
  },
  "focused": 1,
  "names": { "2": "Behavior" }
}
```

Current pane kinds/settings are:

| `kind` | Saved `state` |
| --- | --- |
| `text` | `topic`, `pretty` |
| `image` | `topic`, `overlays` |
| `parameter` | `node`, `path`, `layer`; unsent value edits are excluded |
| `map` | `current_plot_type`, `zoom_and_pan`, and each layer's saved settings |

Save complex layouts through the UI rather than assembling split/grid settings by hand.
Examples are in `tools/twix/presets`; envelope validation is implemented in
`tools/twix/src/layout/persistence.rs`, and each panel defines its own saved settings.
There is no separate schema-version/migration field, so retain a backup before adapting a preset to a changed format.
Validation limits layouts to 4096 tiles, depth 128 and tile IDs at most `2^20`; references must form a reachable tree with valid active tabs and split shares.
Structural validation precedes panel construction. An unknown/unrestorable pane falls back to Text and is logged, rather than invalidating an otherwise valid tree.

## Panels and keybindings

ROS-Z Twix on main contains Text, Image, Map, and Parameter panels. The Text panel observes one ROS-Z topic through `ros-z-debug` and renders the latest dynamic payload as JSON. The Image panel observes `TimeWrapper<ros2::sensor_msgs::image::Image>` topics, defaults to `inputs/left_image`, and renders the latest raw camera frame. The Parameter panel discovers ROS-Z nodes with remote parameter services, shows full snapshots or selected paths as JSON, and writes selected paths to active layers with revision checks.

### Experimental optimization monitor (matching worktree required)

The **Ball-filter optimization** panel is not available on main or the currently tracked [Alex simulator branch](https://github.com/alexschmander/hulk/tree/motion-inference-simulator). The following workflow documents experimental worktree additions and requires matching simulator, tuner, and Twix source; a remote branch checkout alone does not provide them. See [Simulator](behavior_simulator.md) for the checkout scope.

To monitor automatic simulator filter tuning, start a run with
`./simulator --tune-ball-filter logs/my-ball-run --keep-tuning-open`, then add
**Ball-filter optimization** with the **+** picker and click **Connect to simulator / optimizer**.
This read-only panel connects independently of the main namespace/router to
`tcp/127.0.0.1:7448`, namespace `/ball_tuning`, and observes `tuning/progress` plus
capture truth/estimates. It shows capture phases, training loss, best parameters
and holdout results; it does not launch the optimizer or apply parameters.
Only one local tuning run can own that endpoint at a time. `--keep-tuning-open`
retains final progress until Ctrl-C; reconnect after restoring a saved panel.

In that experimental worktree only, the preset kind is `ball_filter_optimization`, with an empty saved state object. Its connection, live samples and progress history are not restored; it uses its own fixed local connection independently of global panel settings.

### Keybindings

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
