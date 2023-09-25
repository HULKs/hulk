# Main Setup and Compiling for Webots

This section will guide you through the installation of dependencies and compiling the code for the [Webots](https://www.cyberbotics.com/) simulator.
We recommend using [Arch Linux](https://archlinux.org/) or one of it's derivatives such as [Manjaro](https://manjaro.org/).

## Installing Dependencies

Some packages are required to be installed before you can compile and run our code.
Use your distribution's package manager to install the following dependencies:

=== "Arch Linux/Manjaro"

    1. Install dependencies

        ```sh
        yay -S git git-lfs base-devel rustup rsync cmake clang hdf5 opusfile python webots
        ```
        `yay` is used because `webots` is an [AUR](https://aur.archlinux.org/) package.
        Optionally substitute `yay` with your favorite AUR helper.

    1. Install rust toolchain

        ```sh
        rustup default stable
        ```

=== "Ubuntu"

    1. Install dependencies

        ```sh
        sudo apt install git git-lfs build-essential libssl-dev pkg-config libclang-dev rsync cmake libhdf5-dev libopusfile-dev python3 libasound2-dev libluajit-5.1-dev libudev-dev
        ```

    1. Install Webots
        Download webots from [https://cyberbotics.com/](https://cyberbotics.com/) the XXXX.deb file and install it with

        ```sh
        sudo dpkg -i XXXX.deb
        ```

    1. Install rust toolchain

        Visit [https://rustup.rs/](https://rustup.rs/) for up to date instructions.

=== "Fedora"

    1. Install dependencies

        ```sh
        sudo dnf install git git-lfs hdf5-devel clang-devel rsync cmake python luajit-devel libudev-devel opusfile-devel zstd
        ```

    1. Install Webots

        At the moment there are no official Fedora packages, but the archive for Ubuntu has worked out fine in our experience.
        Download webots from [https://cyberbotics.com/](https://cyberbotics.com/) the XXXX.tar.bz (Ubuntu Archive) file and install to a local directory.

        ```sh
        mkdir ~/tools/     # example install location
        cd ~/tools
        tar -xf ...tar.bz  # creates a directory named `webots`

        # symlink to be accessible from the command line
        ln -s ~/tools/webots/webots ~/.local/bin/webots
        ```

    1. Install rust toolchain

        Visit [https://rustup.rs/](https://rustup.rs/) for up to date instructions.

## Acquiring the code

Clone our [HULKs/hulk](https://github.com/HULKs/hulk) repository from GitHub:

```sh
git clone git@github.com:HULKs/hulk
```

## Compiling for Webots

In the root of our repository is a script called `pepsi`. See [pepsi](../tooling/pepsi.md) for details.
Simply execute the build command in the repository root to build a binary for use with Webots.
This will first build the pepsi binary and then start the build process.

```sh
./pepsi build
```

## Running Webots

Once the compilation step is complete, open webots and load the scene at `webots/worlds/penalized.wbt` from the repository.

## Running Webots in external Mode

To not be forced to reload the scene in Webots when rebuilding the controller, you can run in webots `webots/worlds/penalized_extern.wbt` and starting the controller with:

```sh
./pepsi run
```

## Running Behavior Simulator

To be able to run the current behavior simulator files, you have to install lua ```ìnspect``` package, either by downloading and saving it to the lua path (e.g., you hulk repo) or by using a lua package manager.

Afterwards you can run the simulator by executing the following command in your hulk project root folder:
```sh
cargo run --manifest-path=tools/behavior_simulator/Cargo.toml serve tests/behavior/golden_goal.lua
```
The results can be inspected in twix.
