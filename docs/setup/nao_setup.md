# NAO Setup

This section assumes you have a working SDK installed.
See [Nao Image and SDK](./nao_image_and_sdk.md) to learn how to acquire or build one.

## Configuring Team Specific Data

=== "HULKs Members"

    There is nothing to do, all the configuration should be ready to if you cloned the `hulks/nao` repository.

=== "Non HULKs Members"

    ### SSH Keys

    If you built your own SDK, you should have created an SSH key pair.
    The public key is used during image creation and registered as an authorized key in the openssh configuration.
    All of our tools expect the private key to be located at `scripts/ssh_key`.
    If you did not create the SDK yourself, ask your teammates for the key.

    ### Set up Team Number

    In the HULKs code release, the SPL team number is hardcoded in a few places. Change this to your own team number before continuing.

    - `crates/spl_network/src/lib.rs` contains a constant called HULKS_TEAM_NUMBER. You may also wish to rename this constant.
    - `tools/pepsi` contains a bunch of `24`s, however most of them are in comments or CLI command help text.
        - `tools/pepsi/src/parsers.rs` has a default and a check value that use 24 literals.
    - `tools/twix/src/completion_edit.rs` generates IP address suggestions with a hardcoded team number.

    ### Set up Hardware IDs

    The tooling around our framework expects each NAO robot to have a number associated with it's hardware IDs.
    This number also determines the last octet of a robot's IP addresses.
    For example robot number `21` will always have the IPv4 addresses `10.0.X.21` (wireless) and `10.1.X.21` (ethernet) where X is the team number.

    For each robot you must determine it's head and body IDs and enter them in `etc/configuration/hardware_ids.json`.
    This file is used by [pepsi](../tooling/pepsi.md) and other tools to find the hardware ids belonging to a robot number.

## Flashing the Firmware

### Preparing a Flash-Stick

First, the firmware image has to be flashed to a USB stick.
Use `lsblk` to make sure you are overwriting the correct device.
```sh
lsblk
```

All existing data on the target device will be wiped!
Replace `sdX` with the USB device.
```sh
dd if=path-to-nao-image.opn of=/dev/sdX status=progress
```

Finally, run `sync` to make sure all data has actually been written to the stick before unplugging it.
```sh
sync
```

### Flashing the NAO

- Make sure the robot is turned off and a charger is plugged in to prevent a sudden loss of power during the process.
- Plug the prepared USB stick into the back of the NAO's head.
- Hold the chest button for about 5 seconds until it starts glowing, then release immediately.
  The chest button LED should now be flashing rapidly.
- Wait for the flashing process to finish
- The new firmare should be installed now.


## Compiling for NAO

`./pepsi build --target nao` will compile the code for use on the NAO with the `incremental` cargo profile.

## Uploading to the NAO

`./pepsi upload <number>` will first compile the code with the same `incremental` cargo profile and the upload the binary and configuration files to the robot with the specified number.

## Remote Shell Access

`./pepsi shell <number>` establishes an SSH connection and presents an interactive shell to the user.
