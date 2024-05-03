# NAO Setup

This section assumes you have a working development environment, and can successfully run `pepsi --help`.
See [Development Environment](./development_environment.md) to learn how to setup your environment.

!!! warning

    Make sure a RoboCupper image has been flashed before flashing the first Yocto image, since the latter does not flash the chestboard (which needs up-to-date software).
    This step is not required for flashing subsequent Yocto images.

## Flashing the Firmware

You can flash the firmware both using [pepsi](../tooling/pepsi.md) or manually using an USB stick.
Flashing with pepsi is the preferred option.

### Using `pepsi gammaray`

Pepsi automatically downloads the latest configured release of the HULKs-OS image.
To flash a robot use:

```sh
pepsi gammaray <NAO>
```

Where `<NAO>` is any configured NAO number or a full IP address.

??? info "Alternatively: Using an USB Stick"

    #### Preparing the Stick

    First, the firmware image has to be written to the USB stick.
    Use `lsblk` to find the device that represents the USB stick.

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

    #### Flashing the NAO

    - Make sure the robot is turned off and a charger is plugged in to prevent a sudden loss of power during the process.
    - Plug the prepared USB stick into the back of the NAO's head.
    - Hold the chest button for about 5 seconds until it starts glowing blue, then release immediately.
      The chest button LED should now be flashing rapidly.
    - Wait for the flashing process to finish
    - The robot reboots at the end of the flashing process.

### Checking for success

When the flash process was successfull, the robot boots up and presents with red [Knight Rider](https://media.giphy.com/media/v1.Y2lkPTc5MGI3NjExYmloOHd2NDJtcjkzaWFqZ2t2c2xjeWZuZjZlZGdueTNxOGUzdXA5byZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/4ViH9IuRZO2wo/giphy.gif) eyes.
The new HULKs-OS is now installed and the NAO is waiting for the robotics software.

## Remote Shell Access

`./pepsi shell <NAO>` establishes an SSH connection and presents an interactive shell to the user.
