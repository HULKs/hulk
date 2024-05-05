# Configuring Team Specific Data

!!! todo

    This page has to be improved.

## HULKs Members

There is nothing to do, all the configuration should be ready to go if you cloned the `hulks/hulk` repository.

## Non HULKs Members

### Set up Team Number

In the HULKs code release, the SPL team number is hardcoded in a few places. Change this to your own team number before continuing.

- `crates/spl_network/src/lib.rs` contains a constant called `HULKS_TEAM_NUMBER`. You may also wish to rename this constant.
- `tools/pepsi` contains a bunch of `24`s, however most of them are in comments or CLI command help text.
    - `tools/pepsi/src/parsers.rs` has a default and a check value that use 24 literals.
- `tools/twix/src/completion_edit.rs` generates IP address suggestions with a hardcoded team number.
- `etc/parameters/hardware.json` has an attribute called spl for team communication hardcoded to 10024 (10000 + team number).

### Set up Hardware IDs

The tooling around our framework expects each NAO robot to have a number associated with it's hardware IDs.
This number also determines the last octet of a robot's IP addresses.
For example robot number `21` will always have the IPv4 addresses `10.0.X.21` (wireless) and `10.1.X.21` (ethernet) where X is the team number.

For each robot you must determine it's head and body IDs and enter them in `etc/parameters/hardware_ids.json`.
This file is used by [pepsi](../tooling/pepsi.md) and other tools to find the hardware ids belonging to a robot number.

