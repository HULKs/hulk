# Team and Robot Configuration

## Team Number

`etc/parameters/team.toml` contains the team number used by Booster provisioning.
HULKs uses `24`. Outside teams must also update remaining source-level constants:

- `crates/hsl_network_messages/src/lib.rs`: `HULKS_TEAM_NUMBER`, used in
  GameController message conversion.
- `crates/argument_parsers/src/lib.rs`: team octet `24` in robot-address
  shortcuts and conversion from addresses back to robot numbers.
- `tools/pepsi/src/gammaray.rs`: the wired gateway `10.1.24.1`.

If using older tools or simulators, inspect their team constants as well.
`tools/twix-legacy/src/main.rs` contains legacy team-24 address suggestions;
current Twix selects a ROS-Z namespace instead of those address suggestions.
Changing `team.toml` alone does not update source-level hardcoding.

## Register a Booster Robot

Obtain the Jetson serial number from the robot:

```sh
./pepsi shell <initial-IP> "jetson_release -s"
```

Pepsi provisioning extracts the numeric `Serial Number:` value. Enter that
value as a string in `etc/parameters/team.toml`, alongside a unique robot number
and hostname. The schema is:

```toml
team_number = 24

[[robots]]
number = 42
hostname = "Ernst-Guenther"
id = "1421625075354"
```

Use the actual serial number for your robot. These are Jetson IDs, not NAO head
and body IDs. Provisioning and runtime startup both fail if the ID is absent
from this configuration. See [Booster setup](booster_setup.md) for initial access.

## Network Addresses

Provisioning uses the team number and robot number to generate these addresses:

| Interface / network | Address |
| --- | --- |
| Ethernet | `10.1.<team>.<robot>/24` |
| `HSL_A` | `10.107.<team>.<robot>/16` |
| `HSL_B` | `10.108.<team>.<robot>/16` |
| `HSL_C` | `10.109.<team>.<robot>/16` |
| Other configured HSL Wi-Fi networks | `10.0.<team>.<robot>/16` |

Pepsi's number shortcuts are independently hardcoded: `42` resolves to
`10.1.24.42`, and `42w` to `10.0.24.42`. They do not consult `team.toml` or the
selected Wi-Fi network. Use a full IP address on `HSL_A`–`HSL_C` or with a
different team number unless you have updated the parser. See
[WiFi](../operating_system/wifi.md) for network selection.

## ROS-Z Namespace and Parameter Layers

The runtime receives `HARDWARE_ID` from the Jetson serial number, looks up the
robot number in `team.toml`, and uses that number as its namespace, e.g. `/42`.
Twix takes an absolute namespace such as `/42`, not bare `42`.

`hulk_ros_z` loads parameter directories in this order:

1. `<parameter-root>/base`
2. `<parameter-root>/location/<location>`
3. `<parameter-root>/robot/<hardware-id>`

Later layers override earlier layers. Parameters are JSON5 files, typically
named after a node; shared parameters are in `global.json5`. The robot launcher
uses `--parameter-root etc/parameters` and `--location default-location`.
Inspect the location name and directory in your checkout when configuring a
deployment: `pepsi location` currently manages a `default_location` name with
an underscore, while the launcher uses a hyphen. Resolve that mismatch for your
deployment; the two names are not interchangeable.

Set each playing robot's player number before upload:

```sh
./pepsi playernumber 42:1 43:2
```

This writes `etc/parameters/robot/<id>/global.json5` while preserving other
existing fields. The base player number is `"Three"`; cloning the repository
does not assign unique player numbers to multiple robots.

`./pepsi pregame` reads `deploy.toml` to configure the playing robots and deploy
them. Consult `./pepsi pregame --help` and `deploy.toml.example` when preparing
a game. Continue with [uploading HULK](upload.md).
