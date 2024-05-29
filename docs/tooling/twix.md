# Configuration

Twix loads a user configuration file on startup. The location of the configuration file depends on your platform:
- Linux: `/home/alice/.config/hulks/twix.toml`
- Windows: `C:\Users\Alice\AppData\Roaming\hulks\twix.toml`
- MacOS: `/Users/Alice/Library/Application Support/hulks/twix.toml`

See the example config further down on this page for the general format.

After loading the user configuration, it is merged with the default values, which are defined in `tools/twix/default.toml`.

## Keybindings

You can customize your keybindings by modifying the `[keys]` table in your user configuration file.
Each table entry is of the form `(trigger) = "(action)"`.
A keybind trigger is a hyphen-separated list of zero or more modifiers, terminated by a key.

See [here](https://github.com/emilk/egui/blob/c1eb3f884db8bc4f52dbae4f261619cee651f411/crates/egui/src/data/key.rs#L298-L413)
for a complete list of bindable keys.

Possible modifiers are `A` (Alt), `C` (Ctrl), and `S` (Shift).
When binding a single-letter key, it is also possible and preferred to specify "Shift" by capitalizing the letter, e.g. `C-T` for Ctrl+Shift+T.

Example keybind triggers:
- `C-t`: Ctrl + T
- `C-T`: Ctrl + Shift + T
- `A-S-Esc`: Alt + Shift + Escape

Possible actions are:
| Action         | Description                                      |
|----------------|--------------------------------------------------|
|`close_tab`     | Close current tab                                |
|`duplicate_tab` | Duplicate current tab                            |
|`focus_above`   | Move focus up                                    |
|`focus_address` | Focus the NAO address field                      |
|`focus_below`   | Move focus down                                  |
|`focus_left`    | Move focus left                                  |
|`focus_panel`   | Focus the panel selector                         |
|`focus_right`   | Move focus right                                 |
|`open_split`    | Split the current surface along the longest axis |
|`open_tab`      | Open a new tab in the current surface            |
|`reconnect`     | Reestablish the connection to the NAO            |

## Example configuration

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
