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

If an old saved layout fails to load, or if you want to reset the current panel setup, start Twix with `--clear`.

Twix checks the local repository version at startup and warns when the running binary is older than the checked-out `tools/twix/Cargo.toml` version. Use `--repository-root <path>` to point that check at a different checkout.

ROS-Z Twix currently contains Text, Image, and Parameter panels. The Text panel observes one ROS-Z topic through `ros-z-debug`, renders the latest dynamic payload as JSON, and shows sample metadata. The Image panel observes `TimeWrapper<ros2::sensor_msgs::image::Image>` topics, defaults to `inputs/left_image`, and renders the latest raw camera frame. The Parameter panel discovers ROS-Z nodes with remote parameter services, shows full snapshots or selected paths as JSON, and writes selected paths to active layers with revision checks.

The Text panel's **Field** selector displays a subtree of the subscribed message. Leave it empty or choose **Whole message** to display the entire payload. Enter a path and press Enter, or choose a completion:

- `pose.x` selects a nested field.
- `joints[3].position` selects a field inside an array or sequence element. Indices start at zero.
- `state::Walking.speed` selects a field in an enum variant's payload. The selected value is unavailable while another variant is active.
- `"field.with.dots"` selects a field whose name contains punctuation. Field and variant names can be JSON-quoted.

Present optionals are unwrapped when continuing through a path. Absent optionals and missing sequence elements appear as unavailable values; invalid paths show an error. Selecting an optional or enum itself displays its full value. Map entries are not traversable in this first version, but the whole map can be displayed.

Completions use the schema from the latest received sample, including fields in absent optionals and inactive enum variants. Sequence index suggestions use the current length and show at most 64 indices; larger indices can be entered manually. Changing the field does not reconnect the topic subscription. The topic and field path are saved separately with the panel layout, and displayed timestamps and publication metadata still refer to the original message.

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
