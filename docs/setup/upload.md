# Uploading HULK

This section assumes, you have set up your development environment, and flashed a Yocto image to the NAO.

Uploading the software to a NAO is task of Pepsi.
Use `pepsi --help` to inspect the respective commands and feature flags.
To upload the robotics software to the NAO, just run:

```sh
pepsi upload <NAO>
```

Pepsi takes care of downloading and installing the SDK, calling `cargo` to trigger compilation, and uploading the software to the robot.
When successful, you are presented with a green tick in your shell, and the robot is showing rotating rainbow eyes.

## SDK Management

Pepsi automatically checks for the latest SDK configured in the repository and installs it if necessary.
To more specifically manage your SDK installation, we provide the `sdk` subcommand for Pepsi.
Have a look at

```sh
pepsi sdk --help
```
