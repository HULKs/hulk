# Aliveness

Aliveness is a system for querying status information from NAOs in the network. It consists of two parts: The service running on the NAOs and a client for sending aliveness requests to the network and processing answers.

## Information available via aliveness

The following information can be queried from NAOs connected via Ethernet:

-   Hostname
-   Current HULKs-OS version
-   States of the systemd services for HAL, HuLA, HULK and LoLA
-   Battery charge state and current
-   Head ID
-   Body ID
-   Name of the interface the beacon is received from (currently always enp4s0)

## Aliveness service

The aliveness service is built together with the HULKs-OS image and included in it. It is started upon the first connection with the network via Ethernet and listens for all messages send to the multicast address `224.0.0.42` as well as its own IP address.

When receiving a UDP packet with content `BEACON`, it responds by sending the above described information encoded via JSON to the sender.

## Aliveness client

Pepsi includes a fully featured aliveness client with multiple verbosity levels and export options, see [here](./pepsi.md#aliveness) for further information.

Example usage:

```
./pepsi aliveness summary
./pepsi aliveness services --json
./pepsi aliveness battery --timeout 500
./pepsi aliveness all 27 32
```

When executing any of the aliveness subcommands in pepsi, it will send the aforementioned beacon message to the multicast address or to a list of NAO IP addresses. It then collects all responses within a timeout and filters their content according to the chosen verbosity level.

## Potential firewall issues

When no NAO addresses are specified, the beacon is send via multicast and the answers are received via unicast.
Since the answers are from a different IP addresses, most firewalls may block them.

In this case, the user has change their firewall settings to allow the incoming messages, e.g. for ufw by adding the following rule:

```
ufw allow proto udp from 10.1.24.0/24
```
