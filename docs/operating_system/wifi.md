# WiFi and Network Addresses

Current Booster provisioning uses **NetworkManager** and `nmcli`. Gammaray writes profiles under
`/etc/NetworkManager/system-connections/` for `HSL_A`–`HSL_J` and `HSL_HULKs`.
Profiles use interface `wlP1p1s0`, manual IPv4 addresses, disabled IPv6,
passphrase `HSL?!HSL?!`, and initially disabled autoconnect.

## Select a Network

```sh
./pepsi wifi list <robot-IP>
./pepsi wifi scan <robot-IP>
./pepsi wifi status <robot-IP>
./pepsi wifi set HSL_A <robot-IP>
./pepsi wifi set None <robot-IP>
```

`wifi set` enables autoconnect for the selected configured profile and disables
it for the others, then brings the selected connection up. `None` disables
autoconnect for all these profiles and disconnects the Wi-Fi interface if connected.
Use Ethernet access when changing wireless connectivity.

## Address Assignment

The team and robot numbers in `etc/parameters/team.toml` are used at provisioning:

| Network | IPv4 address |
| --- | --- |
| Ethernet (`Wired connection 2`) | `10.1.<team>.<robot>/24` |
| `HSL_A` | `10.107.<team>.<robot>/16` |
| `HSL_B` | `10.108.<team>.<robot>/16` |
| `HSL_C` | `10.109.<team>.<robot>/16` |
| `HSL_D`–`HSL_J`, `HSL_HULKs` | `10.0.<team>.<robot>/16` |

Ethernet provisioning retains `192.168.10.102/24` for robot services and currently
sets gateway `10.1.24.1`. Updating `team.toml` does not change that hardcoded gateway.

Pepsi number shortcuts resolve to team 24 on `10.1` (bare number) or `10.0`
(number followed by `w`). They do not follow the selected profile, so use full
addresses on `HSL_A`–`HSL_C` and for other team numbers.
See [team configuration](../setup/configure_team.md) for remaining constants.

For host-side inspection:

```sh
./pepsi shell <robot-IP> "nmcli connection show"
./pepsi shell <robot-IP> "ip address"
```
