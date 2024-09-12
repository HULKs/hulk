# Setup Development Environment

This section will guide you through the setup of your development environment to build software for the NAO robot and test your algorithms using our various tools.

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
-   [python3](https://www.python.org/)
-   [which](https://carlowood.github.io/which/)
-   [zstd](http://www.zstd.net/)
-   [xz](https://tukaani.org/xz/)
-   [file](https://darwinsys.com/file/)
-   [rsync](https://rsync.samba.org/)

=== "Arch Linux"

    ```sh
    sudo pacman -S git git-lfs clang python3 which zstd xz file rsync
    ```

=== "Fedora"

    ```sh
    sudo dnf install git git-lfs clang python3 which zstd xz file rsync alsa-lib-devel
    ```

=== "Ubuntu"

    ```sh
    sudo apt install git git-lfs clang python3 zstd xz-utils file rsync
    ```

### OpenVINO™ runtime for Neural Networks

You will also need to install the OpenVino:tm: runtime for Webots. The HULKs SDK already contains the runtime for use with the NAOs.

-   [Installation Instructions (Linux)](https://docs.openvino.ai/2024/get-started/install-openvino/install-openvino-linux.html)

If you are using a non-linux operating system (e.g. macOS or Windows), you additionally have to install [docker](https://docs.docker.com/engine/install/).

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

```sh
git clone https://github.com/hulks/hulk.git
```

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
    cargo install --path tools/pepsi
    ```

    Pepsi is subsequently installed at `~/.cargo/bin/pepsi`.
    Don't forget to update it from time to time by reinstalling it to get the latest features and bugfixes. <br> <br>
    The same can also be done for [twix](../tooling/twix.md), our debug tool.
