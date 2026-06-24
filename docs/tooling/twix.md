# Installation

Twix doesn't have to be installed to be used, it can be directly run from the repository with `./twix` (just as pepsi).
But you can also install it into your local system to conveniently use it without rebuilding:

```
./pepsi install twix
```

Twix is subsequently installed at `~/.cargo/bin/twix`. <br>

!!! tip

    Don't forget to update it from time to time by reinstalling it to get the latest features and bugfixes.

# ros-z Backend

Twix connects to a Zenoh router and inspects native ros-z graph topics. The default router endpoint is `tcp/127.0.0.1:7447`.

Use the top-bar router field to switch endpoints at runtime. Disable **Keep connected** to keep Twix open without an active router connection; panels show the disconnected state until reconnecting succeeds.

Select the robot namespace in the top bar. A namespace such as `/42` makes panel topic selectors relative to that robot. For example, `ground_to_field` subscribes to `/42/ground_to_field`.

Twix does not translate old dotted debug paths such as `WorldState.main_outputs.ground_to_field`. Pick native ros-z topics from the topic selector instead.

Twix is read-only in this version. The live UI exposes Text, Plot, Enum Plot, and Map panels. Saved tabs for write panels, remote control, parameter editing, or simulator panels are preserved but render an unsupported message. STOP and other removed controls are unavailable in this milestone.

# Configuration

Twix loads a user configuration file on startup. The location of the configuration file depends on your platform:

-   Linux: `/home/alice/.config/hulks/twix.toml`
-   Windows: `C:\Users\Alice\AppData\Roaming\hulks\twix.toml`
-   MacOS: `/Users/Alice/Library/Application Support/hulks/twix.toml`

See the example config further down on this page for the general format.

After loading the user configuration, it is merged with the default values, which are defined in `tools/twix/config_default.toml`.

## Keybindings

You can customize your keybindings by modifying the `[keys]` table in your user configuration file.
Each table entry is of the form `(trigger) = "(action)"`.
A keybind trigger is a hyphen-separated list of zero or more modifiers, terminated by a key.

See [here](https://github.com/emilk/egui/blob/c1eb3f884db8bc4f52dbae4f261619cee651f411/crates/egui/src/data/key.rs#L298-L413)
for a complete list of bindable keys.

Possible modifiers are `A` (Alt), `C` (Ctrl), and `S` (Shift).
When binding a single-letter key, it is also possible and preferred to specify "Shift" by capitalizing the letter, e.g. `C-T` for Ctrl+Shift+T.

!!! example "Example keybind triggers"

    -   `C-t`: Ctrl + T
    -   `C-T`: Ctrl + Shift + T
    -   `A-S-Esc`: Alt + Shift + Escape

Possible actions are:

<!-- prettier-ignore -->
| Action         | Description                                      |
|----------------|--------------------------------------------------|
|`close_tab`     | Close current tab                                |
|`duplicate_tab` | Duplicate current tab                            |
|`focus_above`   | Move focus up                                    |
|`focus_below`   | Move focus down                                  |
|`focus_left`    | Move focus left                                  |
|`focus_panel`   | Focus the panel selector                         |
|`focus_right`   | Move focus right                                 |
|`no_op`         | Do nothing (used to unbind a key)                |
|`open_split`    | Split the current surface along the longest axis |
|`open_tab`      | Open a new tab in the current surface            |
|`close_all`     | Close all tabs and windows                       |

!!! example "Example configuration"

    ```toml
    # Remap navigation keys for non-vim users
    [keys]
    C-h = "no_op"
    C-j = "no_op"
    C-k = "no_op"
    C-l = "no_op"

    C-Left = "focus_left"
    C-Down = "focus_below"
    C-Up = "focus_above"
    C-Right = "focus_right"
    ```
