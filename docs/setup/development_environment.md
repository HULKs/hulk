# Setup Development Environment

This section will guide you through the setup of your development environment to build software for the NAO robot and test your algorithms using our various tools.

## Operating System

At HULKs, we mainly use Linux for development.
If you're new to Linux, don't worry, at HULKs there are many people who can help you getting started.

??? info "Choosing the correct Linux Distribution"

    ![How-To-Chose-Os](./how_to_choose_os.jpg)

    All joking aside, don't be persuaded to install Arch Linux, unless of course you want to delve into the depths of Linux and broaden your horizons.

## Installing Rust

We require a latest stable release of the Rust toolchain to build our tools.
Visit [https://rustup.rs/](https://rustup.rs/) for up to date instructions on how to install `rustup` for your machine.

## Installing Dependencies

Our software requires a few dependencies to be installed before you can compile, upload, and run our code.
Most of these dependencies are needed for the compilation of local tools.
All dependencies needed for a cross-compilation for the NAO are included in the NAO SDK.
Use your distribution's package manager to install the following dependencies:

-   [Git](https://git-scm.com/), [Git LFS](https://git-lfs.com/)
-   [clang](https://clang.llvm.org/)
-   [cmake](https://cmake.org/)
-   [python3](https://www.python.org/)
-   [which](https://carlowood.github.io/which/)
-   [zstd](http://www.zstd.net/)
-   [xz](https://tukaani.org/xz/)
-   [file](https://darwinsys.com/file/)
-   [rsync](https://rsync.samba.org/)
-   [opusfile](https://opus-codec.org/)
-   [hdf5](https://www.hdfgroup.org/solutions/hdf5/)
-   [luajit](https://luajit.org/)
-   [systemd](https://www.freedesktop.org/wiki/Software/systemd/)

=== "Arch Linux"

    ```sh
    sudo pacman -S git git-lfs clang cmake python3 which zstd xz file rsync alsa-lib opusfile hdf5 luajit systemd-libs
    ```

=== "Fedora"

    ```sh
    sudo dnf install git git-lfs clang cmake python3 which zstd xz file rsync alsa-lib-devel opusfile-devel hdf5-devel systemd-devel luajit-devel
    ```

=== "Ubuntu"

    ```sh
    sudo apt install git git-lfs clang cmake python3 zstd xz-utils file rsync libasound2-dev libopusfile-dev libhdf5-dev libsystemd-dev libluajit-5.1-dev pkg-config
    ```

If you are using a non-linux operating system (e.g. macOS or Windows), you additionally have to install [docker](https://docs.docker.com/engine/install/).

??? "If you want to use our simulator Webots"

    Usually, the Webots simulator is **not needed** for normal development.
    If you want to use it, you can install it with your packet manager or download it from the [official website](https://cyberbotics.com/).
    You will also need to install the OpenVino™ runtime. The HULKs SDK already contains the runtime for use with the NAOs.

    -   [Installation Instructions (Linux)](https://docs.openvino.ai/2024/get-started/install-openvino/install-openvino-linux.html)

## Cloning the Repository

We use Git to manage all our software.

??? info "Git Setup: If you haven't used Git before"

    Git is a free and open source distributed version control system.

    **First**, install Git (see above).

    The **second** thing you should is to set your user name and email address.
    This is important because every Git commit uses this information, and it’s baked into the commits you start creating:

    ```
    git config --global user.name "<your-name>"
    git config --global user.email "<your-email>"
    ```

    And **third**, setup authentication with GitHub
    You can access and write data in repositories on GitHub.com using SSH (Secure Shell Protocol).
    When you connect via SSH, you authenticate using a private key file on your local machine.
    You can follow this [guide](https://docs.github.com/en/authentication/connecting-to-github-with-ssh/generating-a-new-ssh-key-and-adding-it-to-the-ssh-agent) to generate and add a key to GitHub.
    If you already have a key, you can skip the part about generating a new one, and simply add your existing key to GitHub.

To download the repository, run `git clone https://github.com/hulks/hulk.git` in the terminal.

!!! tip

    It's common to not do this in your home directory, but in a separate folder.
    Most people use a `~/worktree` directory for this.
    To create this, run `mkdir ~/worktree` and then `cd ~/worktree`.
    Now you can execute the `git clone` command from above.

## Build [Pepsi](../tooling/pepsi.md)

Pepsi is our main tool to interact with the repository, configure NAOs, and upload the software to the robot.
For a more in depth overview and introduction to Pepsi, consult [../tooling/pepsi.md].

For now, it is sufficient to know that Pepsi takes care of building the source code and also uploading it to the NAO.
This includes downloading and installing the SDK.

To build and run Pepsi from source, use

```sh
./pepsi
```

This downloads and builds all dependencies for the workspace and displays the help page of Pepsi.

!!! tip

    You can also install Pepsi into your local system to conveniently use it without rebuilding:

    ```
    ./pepsi install pepsi
    ```

    Pepsi is subsequently installed at `~/.cargo/bin/pepsi`.
    Don't forget to update it from time to time by reinstalling it to get the latest features and bugfixes. <br> <br>
    The same can also be done for [twix](../tooling/twix.md), our debug tool.
