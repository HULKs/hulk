# Aliveness

Aliveness is a system for querying status information from NAOs in the network. It consists of two parts: The service running on the NAOs and a client for sending aliveness requests to the network and processing answers.

## Information available via aliveness

The following information can be queried from NAOs connected via Ethernet:

- Hostname
- Current HULKs-OS version
- States of the systemd services for HAL, HULK and LoLA (HuLA is not a separate reported field)
- Battery charge state and current
- Head ID
- Body ID
- Wireless network name
- Joint temperatures
- Name of the interface used by the aliveness service

Body ID, head ID, battery, wireless network and temperature fields are optional and may be unavailable.
The service defaults to `eth0`; its first command-line argument can select another interface, so the reported name depends on the installed service configuration.
These NAO-oriented telemetry fields do not imply that a K1 supplies the same hardware information.

## Aliveness service

The NAO aliveness service is built together with the HULKs-OS image. It watches the configured interface and joins the multicast group when that interface has an IPv4 address.
It listens for UDP messages on port `4242`, including multicast address `224.0.0.42` and unicast queries.

When receiving a UDP packet with content `BEACON`, it responds by sending the above described information encoded via JSON to the sender.

## Aliveness client

Pepsi includes a fully featured aliveness client with different verbosity levels and export options, see [here](./pepsi.md#aliveness) for further information.

Example usage:

```
./pepsi aliveness
./pepsi aliveness 27 32
./pepsi aliveness --json
./pepsi aliveness --timeout 500 -v
```

When executing any of the aliveness subcommands in pepsi, it will send the aforementioned beacon message to the multicast address or to a list of NAO IP addresses. It then collects all responses within a timeout and filters their content according to the chosen verbosity level.

## Potential firewall issues

When no NAO addresses are specified, the beacon is sent via multicast and the answers are received via unicast.
Since the answers are from a different IP addresses, most firewalls may block them.

In this case, the user has change their firewall settings to allow the incoming messages, e.g. for ufw by adding the following rule:

```
ufw allow proto udp from 10.1.24.0/24
```
