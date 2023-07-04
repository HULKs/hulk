# Pepsi

Pepsi is a multi-tool we use for anything related to the code or the NAO robots.
It can be used to build the code, set up configuration parameters for a game, deploy to a robot or simply open a remote shell.

This page is only meant as a general overview of pepsi's subcommands.
For detailed usage instructions, run `pepsi --help` or `pepsi <subcommand> --help`.

## Typical Webots Workflow

This is pretty simple. Open Webots, load the `webots/worlds/penalized_extern.wbt` world file and execute

```bash
./pepsi run
```

in your terminal. This will build (if necessary) and then run the webots binary.
The simulation is paused automatically until the binary starts.

## Typical NAO Workflow

```bash
./pepsi upload <number or IP>
```

This command does the following:

- checks if a toolchain is installed, downloads, and installs one if necessary
- builds the code for the NAO target
- uploads binary, configuration parameters, motion files, neural networks, etc. to the NAO(s)
- restarts HULK service on the NAO(s)

## Interaction with the NAO

NAOs are identified either by IP or by number.
Numbers are converted to IPs as follows:

- `{number}` -> `10.1.24.{number}`
- `{number}w` -> `10.0.24.{number}`

Many subcommands can act on multiple robots concurrently.

`upload` builds a binary for the NAO target, and then uploads it and parameter files to one or more robot.

`wireless`, `reboot`, `poweroff`, and `hulk` directly interact with the robot(s), whereas `communication`, and `playernumber` only change the local configuration parameters.

`pregame` combines deactivating communication (to avoid sending illegal messages), assigning playernumbers, setting a wifi network, uploading, and restarting the HULK service.

`logs` or and `postgame` can be used after a (test-)game to download logs, the latter also shuts down the HULKs binary and disables wifi.

`gammaray` is used for flashing a HULKs-OS image to one or more robots.

## Build Options

For subcommands that build a binary, you can specify a target and a build profile.
These include `build`, `run`, `check`, and `clippy`.
However `upload` and `pregame` only supports a profiles, since it doesn't make sense to upload a webots binary to the nao.

## Aliveness

Using the `aliveness` subcommand, pepsi can query information from NAOs connected via ethernet. By default, only irregular information like non-active services, outdated HULKs-OS versions and battery charge levels below 95% are displayed. Using `-v`/`--verbose` or `-j`/`--json`, you can retrieve all information available via aliveness in either a human- or machine-readable format.

You can also set a timeout via `-t`/`--timeout` (defaulting to 200ms) and specify NAO addresses (e.g. `22` or `10.1.24.22`) for querying the aliveness information only from specific NAOs.

Further information on the information available via aliveness as well as the details to the protocol can be found [here](./aliveness.md).

## Shell Completion

Shell completions can be generated using the `completions` subcommand.

Example:

```bash
./pepsi completions zsh > _pepsi
```

Refer to your shell's completion documentation for details.

The shells completions for fish, zsh and bash include dynamic suggestions for all pepsi subcommands taking a NAO address as an argument (e.g. `pepsi upload`).
Those suggestions are retrieved using the aliveness service and require a version of pepsi to be installed in the `PATH`, e.g. by using

```
cargo install --path tools/pepsi
```

and adding `~/.cargo/bin` to the `PATH`.
