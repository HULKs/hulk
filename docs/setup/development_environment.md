# Development Environment

## Operating System

At HULKs, we mainly use Linux for development.
If you're new to Linux, don't worry, at HULKs there are many people who can help you getting started.

??? info "Choosing the correct Linux Distribution"

    ![How-To-Chose-Os](./how_to_choose_os.jpg)

    All joking aside, don't be persuaded to install Arch Linux, unless of course you want to delve into the depths of Linux and broaden your horizons.

## Prerequisites

On x86_64 Linux, install [Nix](https://nixos.org/download/), Git, and Git LFS.
Enable `nix-command` and `flakes` in your Nix configuration.
To deploy to a robot, make sure `ping` is available on the host, and install `curl` if you need to prepare a runtime bundle.

## Cloning the Repository

??? info "Git setup for new contributors"

    Set your name and email so Git can attribute your commits:

    ```sh
    git config --global user.name "<your-name>"
    git config --global user.email "<your-email>"
    ```

    If you don't already have an SSH key for GitHub, follow [GitHub's SSH setup guide](https://docs.github.com/en/authentication/connecting-to-github-with-ssh/generating-a-new-ssh-key-and-adding-it-to-the-ssh-agent).

```sh
git lfs install
git clone https://github.com/HULKs/hulk.git
cd hulk
```

!!! tip

    Many HULKs members keep their checkouts in a separate directory such as `~/worktree`.

## Build Pepsi

```sh
nix develop
./pepsi --help
```

`./pepsi` and `./twix` use the Cargo on your `PATH`. Run `nix develop` first if you don't have Rust installed.
Outside the Nix shell, your Rust toolchain must meet the version in `Cargo.toml`, and Twix also needs the GUI libraries.
Pepsi uses Cargo on your `PATH` for workstation builds by default. Run `nix develop` first for builds that need libraries supplied by the flake, or select the workstation Nix shell with `--env native`.

For robot builds and deployment, see the [Pepsi guide](../tooling/pepsi.md).

!!! tip

    To install Pepsi or [Twix](../tooling/twix.md) from the checkout without Nix, run these commands outside `nix develop`:

    ```sh
    ./pepsi install pepsi
    ./pepsi install twix
    ```

    These commands use the Cargo and linker on your `PATH` without opening a Nix shell.
    Use a host Rust toolchain that meets the version in `Cargo.toml`.
    For Twix, also provide `pkg-config`, libclang for bindgen, and the native GUI libraries for your distribution.
    Add `~/.cargo/bin` to `PATH` if needed, and reinstall after updating the checkout to pick up changes.

### Inference runtime compatibility

Nix supplies ONNX Runtime for both workstation and robot builds.
The Rust workspace uses `ort` rc.13 with `api-21` to keep provider selection explicit, so revalidate the runtime before enabling default features or `api-22` and newer.
Use `GraphOptimizationLevel::All` to retain rc.10's `Level3` behavior; in rc.13, `Level3` selects `ORT_ENABLE_LAYOUT`.
Local CPU validation does not cover Jetson CUDA or TensorRT execution.
