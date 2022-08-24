# Main Setup

This section will guide you through the installation of dependencies and compiling the code for the [Webots](https://www.cyberbotics.com/) simulator.
We recommend using [Arch Linux](https://archlinux.org/) or one of it's derivatives such as [Manjaro](https://manjaro.org/).

TODO: NAO?

## Installing Dependencies

Some packages are required to be installed before you can compile and run our code.
Use your distribution's package manager to install the following dependencies:

ToDo: Git? Git-lfs?
ToDo: split by common/webots/nao/tooling?

=== "Arch Linux/Manjaro"

    - base-devel
    - rust *(rustup)*
    - rsync
    - cmake
    - clang
    - hdf5
    - python
    - webots

=== "Ubuntu"

    - build-essential
    - rust *(rustup)*
    - libssl-dev
    - pkg-config
    - libclang-dev
    - rsync
    - cmake
    - libhdf5-dev
    - python
    - webots

If you installed rust via rustup, make sure to download the latest rust toolchain as well.

```sh
$ rustup default stable
```

## Acquiring the code 

If you are a HULKs member, you should have access to our main repository at on github: [HULKs/nao](https://github.com/HULKs/nao). Otherwise, use our code release at [HULKs/CodeRelease](https://github.com/HULKs/HULKsCodeRelease).

## Compiling for webots

In the root of our repository is a script called `pepsi`. See [pepsi](../tooling/pepsi.md) for details.
Simply execute the build command in the repository root to build a binary for use with Webots.
This will first build the pepsi binary and then start the build process.


```sh
$ pepsi build
```

## Running webots

Once the compilation step is complete, open webots and load the scene at `webots/worlds/penalized.wbt` from the repository.
