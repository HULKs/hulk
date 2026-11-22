# HUlks Level Abstraction (HULA)

HULA is an abstraction layer that connects to LoLA, the by [Aldebaran Robotics](https://corporate-internal-prod.aldebaran.com/) provided Low Level Abstraction of the NAOv6 and serves an interface for other applications.
Unlike LOLA, HULA also supports multiple clients.

![Overview Diagram](./hula-overview-light.svg#only-light)
![Overview Diagram](./hula-overview-dark.svg#only-dark)

## On the Nao

On the NAO, systemd manages the `hula.service`.
It can be stopped / started using standard systemd commands, such as `systemctl start hula`.

## Custom Build

To build HULA for the NAO, use a historical checkout that contains `tools/hula` (for example, `fd0743f0a`) and obtain the [NAO SDK](../setup/nao_image_and_sdk.md#building-the-sdk).
Pepsi's current robot SDK targets Booster, not NAO.

1. Source the SDK by calling `. naosdk/<version>/environment-setup-corei7-64-aldebaran-linux` from the default HULKs folder.
   Be sure to use a POSIX compliant shell such as Bash (not Fish).
2. Use `cargo build --manifest-path tools/hula/Cargo.toml` to compile the HULA binary.
3. Copy the compiled binary from the target folder in the hula folder to the NAO using e.g. `scp`.
4. Connect to the NAO using SSH or `pepsi shell` from the historical checkout
5. Stop the probably already running default hula by calling `systemctl stop hula`
6. Execute the copied hula binary
7. Enjoy 🚀
