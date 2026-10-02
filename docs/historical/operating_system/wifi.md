# WiFi

!!! info "Historical NAO networking"

    This page preserves older NAO/iwd networking instructions. Current Booster networking uses NetworkManager; see [current WiFi](../../operating_system/wifi.md).

    Even within the NAO scope, verify the image revision. At `meta-nao` revision `a41dfa5f79687e7685eda1f04671ab84fb9709e3`, the recipes install `SPL_A`–`SPL_F` profiles with passphrase `Nao?!Nao?!`, and `configure_network` reads `/media/internal/team.toml` and writes `20-wlan.network` and `20-wired.network`. The HSL profiles, JSON ID map, and `80-*` names below describe older configuration.

The WiFi is supplied by a Qualcomm Atheros AR9462 and configured via the [iNet Wireless Daemon (iwd)](https://iwd.wiki.kernel.org/).
For more information on iwd visit their documentation.

## `iwd` Configuration

The iwd service can be manually configured using the command line interface tool `iwctl`.
For persistent configuration iwd stores `*.psk` files for every known SSID at `/var/lib/iwd/`.
The yocto distribution installs those `*.psk` files for the network SSIDs *HSL_A* to *HSL_F*.

```sh
[Security]
Passphrase=HSL?!HSL?!

[Settings]
AutoConnect=false
```

Automatic connection is disabled to prevent the Nao to connect to any SPL network in range.
If iwd was tasked to connect to a network once, it tries to reconnect to that same SSID until the daemon is instructed to disconnect.

The iwd is also able configure IP settings and run DHCP.
This is called *Network Configuration* and disabled via the `/etc/iwd/main.conf`.
IP configuration is and done by [systemd-networkd](https://www.freedesktop.org/software/systemd/man/systemd.network.html).

```sh
[General]
EnableNetworkConfiguration=false
```

## IP Configuration

The Nao's IP address is derived from the robots id number in the `../setup/nao_image_and_sdk.md`.
This follows the pattern `10.{Interface}.{TeamNumber}.{NaoNumber}`.
For the robot 22 of team HULKs this is `10.0.24.22` on the wireless interface and `10.1.24.22` on the wired interface.

Responsible for the configuration is the systemd unit `network-config` running the `/usr/sbin/configure_network` script once per boot.
This script is calculating the IP configuration based on the entries in the `/etc/id_map.json` and generates systemd network configuration files at `/etc/systemd/network/80-wlan.network` and `/etc/systemd/network/80-wired.network`.
