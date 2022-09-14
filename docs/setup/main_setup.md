# Main Setup and Compiling for Webots

This section will guide you through the installation of dependencies and compiling the code for the [Webots](https://www.cyberbotics.com/) simulator.
We recommend using [Arch Linux](https://archlinux.org/) or one of it's derivatives such as [Manjaro](https://manjaro.org/).

## Installing Dependencies

Some packages are required to be installed before you can compile and run our code.
Use your distribution's package manager to install the following dependencies:

=== "Arch Linux/Manjaro"

    1. Install dependencies
    ```sh
    sudo pacman -S git git-lfs base-devel rustup rsync cmake clang hdf5 python webots
    ```
    1. Install rust toolchain
    ```sh
    rustup default stable
    ```

=== "Ubuntu"

    1. Install dependencies
    ```sh
    sudo apt install git git-lfs build-essential rustup libssl-dev pkg-config libclang-dev rsync cmake libhdf5-dev python
    ```
    1. Install Webots
    ```sh
    sudo snap install webots
    ```
    1. Install rust toolchain
    ```sh
    rustup default stable
    ```

## Acquiring the code

=== "HULKs Members"

    If you are a HULKs member, you should have access to our [HULKs/nao](https://github.com/HULKs/nao) repository on GitHub:

    ```sh
    git clone git@github.com:HULKs/nao
    ```

=== "Non HULKs Members"

    If you are not a member of the HULKs club, use our code release at [HULKs/CodeRelease](https://github.com/HULKs/HULKsCodeRelease).

    ```sh
    git clone git@github.com:HULKs/HULKsCodeRelease
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
