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

The Text panel's **Topic** input accepts both a topic and a nested field path. Enter just the topic to display its whole message, or append a field path with a dot and press Enter:

- `detected_objects.inner` selects the wrapped detections.
- `detected_objects.inner[2].bounding_box.confidence` selects the third detected object's confidence. Arrays and sequences use zero-based indices, and paths can continue through nested structs and arrays.
- `status.state::Walking.speed` selects a field in an enum variant's payload. The selected value is unavailable while another variant is active.
- `topic."field.with.dots"` selects a field whose name contains punctuation. Field and variant names can be JSON-quoted.

Topics and fields autocomplete in the same input. Array completions include templates such as `detected_objects.inner[...]`. Selecting a template highlights `...` so you can replace it with an index, move past the closing bracket, and continue into the element's fields. Templates also work when the current array is empty. For a topic whose root is an array, use `topic[2]`.

Press **Ctrl+F** to focus and select the topic input in the active Text or Image panel. Press **Ctrl+Space** in a completion input to open its dropdown without changing the text, including when the input is empty. Use the arrow keys to choose a completion and Enter to apply it.

Present optionals are unwrapped when continuing through a path. Absent optionals and missing sequence elements appear as unavailable values; invalid paths show an error. Selecting an optional or enum itself displays its full value. Map entries are not traversable in this first version, but the whole map can be displayed.

Field completions use the schema from the latest received sample of the selected topic, including fields in absent optionals and inactive enum variants. Select a topic first to browse its fields, or enter a full path directly. Sequence index suggestions use the current length and show at most 64 indices; larger indices can be entered manually. Changing the field does not reconnect the topic subscription. Existing saved topic/field selections restore into the combined input. Displayed timestamps and publication metadata still refer to the original message. For topic names containing dots, the longest matching discovered topic takes precedence when separating topic and field path.

ROS-Z Twix reads keybindings from `hulks/twix-ros-z.toml`. Legacy Twix keeps using `hulks/twix.toml`, so the two tools do not share incompatible keybinding schemas. The default ROS-Z keybindings are:

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

Directional focus selects the nearest visible panel and outlines it. Press `Tab` after moving focus to enter that panel's controls; subsequent `Tab` and `Shift-Tab` presses follow the normal control order.
