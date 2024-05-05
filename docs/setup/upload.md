# Uploading HULK

This section assumes that you have set up your development environment and flashed a Yocto image to the NAO.

Uploading the software to a NAO is done with Pepsi.
See `pepsi --help` for details on the respective commands.
To upload the robotics software to the NAO, just run:

```sh
pepsi upload <NAO>
```

Pepsi takes care of downloading and installing the SDK, calling `cargo` to trigger compilation, and uploading the software to the robot.
When successful, you are presented with a green tick in your shell, and the robot is showing rotating rainbow eyes.

## SDK Management

Pepsi automatically checks for the latest SDK configured in the repository and installs it if necessary.
To manually manage your SDK installation, use the `sdk` subcommand.

```sh
pepsi sdk install --help
```
