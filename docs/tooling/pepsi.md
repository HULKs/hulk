# Pepsi

Pepsi is a multi-tool we use for anything related to the code or the NAO robots.
It can be used to build the code, set up configuration for a game, deploy to a robot or simply open a remote shell.

This page is only meant as a general overview of pepsi's subcommands.
For detailed usage instructions, run `pepsi --help` or `pepsi <subcommand> --help`.

## Typical Webots Workflow

This is pretty simple. Open Webots, load the `webots/worlds/penalized_extern.wbt` world file and execute
```bash
pepsi run
```
in your terminal. This will build (if necessary) and then run the webots binary.
The simulation is paused automatically until the binary starts.

## Typical NAO Workflow

```bash
pepsi upload <number or IP>
```
This command does the following:

- checks if a toolchain is installed, downloads, and installs one if necessary
- builds the code for the NAO target
- uploads binary, configuration, motion files, neural networks, etc. to the NAO(s)
- restarts HULK service on the NAO(s)

## Interaction with the NAO

NAOs are identified either by IP or by number.
Numbers are converted to IPs as follows:

 * `{number}` -> `10.1.24.{number}`
 * `{number}w` -> `10.0.24.{number}`

Many subcommands can act on multiple robots concurrently.

`upload` builds a binary for the NAO target, and then uploads it and configuration files to one or more robot.

`wireless`, `reboot`, `poweroff`, and `hulk` directly interact with the robot(s), whereas `communication`, and `playernumber` only change the local configuration.

`pregame` combines deactivating aliveness & communication (to avoid sending illegal messages), assigning playernumbers, setting a wifi network, uploading, and restarting the HULK service.

`logs` or and `postgame` can be used after a (test-)game to download logs, the latter also shuts down the HULKs binary and disables wifi.

## Build Options

For subcommands that build a binary, you can specify a target and a build profile.
These include `build`, `run`, `check`, and `clippy`.
However `upload` and `pregame` only supports a profiles, since it doesn't make sense to upload a webots binary to the nao.

## Shell Completion

Shell completions can be generated using the `completions` subcommand.

Example:
```bash
pepsi completions zsh > _pepsi
```
Refer to your shell's completion documentation for details.
