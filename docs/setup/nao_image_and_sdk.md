# NAO Image & SDK

TODO: decide whether to backtick all "meta-nao", "meta-hulks", "kas", "BitBake" names
TODO: "code release" or "code-release" or "coderelease"?
TODO: Set up id map json first
TODO: Change team number in meta-hulks

The HULKs use the [Yocto Project](https://yoctoproject.org) for creating flashable OPN images for the NAO and a corresponding software development kit (SDK) for local development targeting the NAO.
The SDK contains a full cross-compilation toolchain that is self-contained and can be easily used on development machines.

## Use an Existing Yocto SDK with the HULKs Code

=== "HULKs Members"

    HULKs members can just use the Pepsi tool (TODO: link to Pepsi) to compile and upload to a booted NAO.
    This needs to be done in the lab or with VPN access to the BigHULK.
    Within the HULKs repository, use the following command to e.g. upload to NAO 42:

    ```sh
    ./pepsi upload 42
    ```

    You will be asked to install the SDK during the compilation process.
    Just choose the defaults if unsure or ask your fellow HULK.

=== "Non HULKs Members"

    Non HULKs members need to copy the Yocto SDK to the local downloads folder to prevent the Pepsi tool (TODO: link to Pepsi) from downloading it during compilation.
    For instructions on how to build the image and SDK refer to the section [Image & SDK Creation](#image-sdk-creation).

    ```sh
    mkdir -p sdk/downloads/
    cp .../HULKs-OS-toolchain-[...].sh sdk/downloads/
    ./pepsi upload 42
    ```

    You will be asked to install the SDK during the compilation process.
    Just choose the defaults if unsure.

## Image & SDK Creation

The Yocto Project leverages [BitBake](https://en.wikipedia.org/wiki/BitBake) as task execution engine and offers an abstraction layer to modify and extend existing build configurations.
Combined with [OpenEmbedded](https://www.openembedded.org/wiki/Main_Page), the entire worktree is structured in several layers (the HULKs use the meta-nao and meta-hulks layers).

### Setup of the Working Directory

For creating the image and SDK, make sure there is at least 100 GB empty disk space available.
Start by cloning the code (if not done before) and setting up a Yocto working directory.
This working directory will contain the layers [HULKs/meta-nao](https://github.com/HULKs/meta-nao) and meta-hulks, a script for running BitBake commands, and the HULKs nao repository.
It will be called `yocto`.
The new `yocto` working directory must have at least 100 GB of empty space available and should not be part of a Git repository.

```sh
mkdir yocto/
cd yocto/
```

=== "HULKs Members"

    HULKs members need the [HULKs/nao](https://github.com/HULKs/nao) repository, and the [HULKs/meta-nao](https://github.com/HULKs/meta-nao) and [HULKs/meta-hulks](https://github.com/HULKs/meta-hulks) layer.
    The rest should already be set up.

    ```sh
    # working directory is still `yocto`
    git clone git@github.com:HULKs/nao
    git clone git@github.com:HULKs/meta-nao
    git clone git@github.com:HULKs/meta-hulks
    ```

=== "Non HULKs Members"

    Non HULKs members need the [HULKs/meta-nao](https://github.com/HULKs/meta-nao) layer and the meta-hulks layer from [HULKs/CodeRelease](https://github.com/HULKs/HULKsCodeRelease) (in the subdirectory `yocto/meta-hulks`).

    ```sh
    # working directory is still `yocto`
    git clone git@github.com:HULKs/HULKsCodeRelease
    mv HULKsCodeRelease nao
    #ln -s nao/yocto/meta-hulks meta-hulks
    #sed -i 's|path: "patches|path: "yocto/meta-hulks/patches|' meta-hulks/kas-project.yml
    cp -r nao/yocto/meta-hulks meta-hulks
    git clone git@github.com:HULKs/meta-nao
    ```

    During the HULKs code release generation, some cryptographic keys are replaced.
    If you want to e.g. connect to a NAO via SSH the following steps need to be done to recreate the keys:

    ```sh
    # working directory is still `yocto`
    ssh-keygen -t ed25519 -C nao@hulk -f nao/scripts/ssh_key
    # Answer `y` when asked to overwrite
    cat nao/scripts/ssh_key.pub > meta-hulks/recipes-connectivity/openssh/openssh/authorized_keys
    ```

    If you wondered, as a non HULKs member you don't need the `meta-hulks/ssh-dir` populated with keys.

    In addition to the SSH keys, also the NAO hardware IDs and wireless network configurations need to be adjusted.
    The hardware IDs can be configured in `meta-hulks/recipes-hulks/network-config/network-config/id_map.json`.
    Wireless networks can be configured at `meta-hulks/recipes-conf/nao-wifi-conf/nao-wifi-conf/*.psk` ([iwd](https://iwd.wiki.kernel.org/) is used).
    If networks are added/removed or names change, the recipe `meta-hulks/recipes-conf/nao-wifi-conf.bb` also needs adjustmenst.

For project setup the [siemens/kas](https://github.com/siemens/kas) framework is used.
To setup kas use the containerized version (podman or docker) via the [kas-container script](https://github.com/siemens/kas/blob/master/kas-container) and store it inside the `yocto` directory.

```sh
wget https://github.com/siemens/kas/raw/master/kas-container
chmod +x kas-container
```

Alternatively setup kas via a python-pip installation, follow the installation steps in the [user guide](https://kas.readthedocs.io/en/latest/userguide.html).

The meta-hulks layer ships a `kas-project.yml` project description file.
This file defines the project structure kas has to setup for the Yocto build phase.
The next step is to download all the referenced repositories in the `kas-project.yml`.

```sh
./kas-container checkout meta-hulks/kas-project.yml
```

The NAO v6 uses LoLA and HAL for communication with the chestboard.
All these binaries and libraries necessary to operate the NAO properly are shipped with the `.opn` RoboCupper image and are **not** included in this repository.
For HULKs members contact our dev-leads and for non HULKs members contact the RoboCup SPL Technical Committee to get this image.
To extract the necessary binaries the `extract_binaries.sh` script is used.
This script fetches all binaries from inside the RoboCupper image and collects them in an archive for the upcoming build phase.
To generate the archive containing the aldebaran binaries run (with root privileges):

```sh
cd meta-nao/recipes-support/aldebaran/
mkdir -p aldebaran-binaries
./extract_binaries.sh -o aldebaran-binaries/aldebaran_binaries.tar.gz nao-2.8.5.11_ROBOCUP_ONLY_with_root.opn
```

### Starting a Build Shell

kas is able to start a shell inside of the build environment.
The `kas-project.yml` of meta-hulks needs to be referenced:

```sh
# working directory is `yocto`
./kas-container shell meta-hulks/kas-project.yml
```

All BitBake and Devtool commands shall be executed from this shell.

### Preparing the Build

The NAO image contains the HULA binary (TODO: link to HULA) which is built from [HULKs/nao](https://github.com/HULKs/nao) or [HULKs/CodeRelease](https://github.com/HULKs/HULKsCodeRelease) (depending
on whether you are a HULKs member or not).
The HULA source code is located in `tools/hula`.
The meta-hulks layer is set up to clone the private [HULKs/nao](https://github.com/HULKs/nao) repository and check out a specific version.
This only works if the kas-container has SSH correctly set up and uses a SSH key that has access to the repository.
Most often it is easier to clone the repository manually and point BitBake to use it.
The following command can be executed within the build environment to do that:

```sh
devtool modify --no-extract hula /work/nao
```

This must be executed at any restart of the build shell.

### Building the Image

Inside of the build shell, the following command will build the NAO image.
The initial build may take multiple hours depending on your hardware and internet access.
BitBake provides advanced caching of the build artifacts which means that future builds are done in minutes depending on the changes.
The cache relies in the `build/sstate-cache` which can be copied from another build directory or shared between machines (
see [Yocto Documentation about Shared State Cache](https://docs.yoctoproject.org/overview-manual/concepts.html#shared-state-cache)).
To build the image run the following command in the build shell:

```sh
bitbake nao-image
```

This generates and executes all necessary tasks and targets to construct a proper `.opn` file.
The initial build phase might take several hours depending on the performance of your build machine and your internet connection.
BitBake uses a very elaborated caching strategy to speed up following builds of targets.
Thus small changes afterwards might only take a few minutes.

As soon as the build has successfully finished, the image can be deployed.
After BitBake ran all tasks up to nao-image, a new `.opn` file is generated in `build/tmp/deploy/images/nao-v6/nao-image-HULKs-OS-[...].ext3.gz.opn`.
The image can now be flashed to a USB flash drive:

```sh
dd if=nao-image-HULKs-OS-[...].ext3.gz.opn.opn of=/dev/sdb status=progress
sync
```

A RoboCupper image needs to be flashed first because the Yocto `.opn` does not flash the chestboard (which needs up-to-date software).
Now flash the NAO with the Yocto image.
The flashing process may take 1-3 minutes.
It is finished if the HULA process displays a red LED animation in the eyes.

### Building the SDK

To be able to compile the HULKs robotics code targeting the NAO platform, the code needs to be cross compiled for the NAO target.
Within the build shell, the following command will build the SDK:

```sh
bitbake -c populate_sdk nao-image
```

This build phase may take several hours.
After a successful build, the SDK is located at `build/tmp/deploy/sdk/HULKs-OS-toolchain-[...].sh`.
To install the SDK run the script and follow the instructions.
Afterwards, you are able to source the build environment and use the respective cross compilers.

### Advanced: Upgrade other Yocto Layers

The Yocto Project and the Poky reference distribution provide a Linux kernel, userland programs, libraries, and other tooling.
All these things are updated in the regular Yocto releases.
To ensure deterministic builds the HULKs freeze versions of all used layers in the `kas-project.yml` files of meta-nao and meta-hulks.

### Advanced: Upgrade Image/SDK Versions and Semantic Versioning

The HULKs use semantic versioning for the Yocto images and SDKs.
This means that versions are increased depending on the severity of changes.
The following policy exists for the HULKs:

-   Images have major, minor, and patch version numbers (e.g. 4.2.3), SDKs have only have major and minor (e.g. 4.2)
-   Same version numbers of images and SDKs are compatible to each other
-   Major changes, refactorings, implementations result in the increase of the major version number
-   Minor changes, additions and iterations result in the increase of the minor version number
-   Changes in the image that do not require SDK recreation, result in the increase of the patch version number (which only requires to create a new image)

Before building new images, the version number needs to be set in `meta-hulks/conf/distro/HULKsOS.conf`.
Only change the `DISTRO_VERSION`, the `SDK_VERSION` is automatically derived from the `DISTRO_VERSION`.
Once new SDKs are deployed at the BigHULKs for HULKs members or in the local downloads directory `sdk/downloads` in the HULKs repository for non HULKs members, the Pepsi tool needs to learn to use the new SDK.
Therefore update the version in `crates/repository/src/lib.rs` in the variable `SDK_VERSION`.
Successive builds with Pepsi will use the new version.

### Advanced: Upgrade Rust Version

Since upgrading the Rust version often requires manual steps, this section describes the approach on how to upgrade and generate the needed patch files.
These instructions can be followed e.g. if a new Rust version is available and a new image/SDK should be created with this new version.
Users that just want to use the current version that we upgraded to should skip this section.
The latest patch set for is included in the meta-hulks layer (in `patches/`) or HULKs code release (in `yocto/meta-hulks/patches/`).

Rust is provided by the poky repository.
The recipes are located in `meta/recipes-devtools/{cargo,rust}`.
The following steps are high-level instructions on how to modify the poky repository.
A patch file can be created after applying these instructions and saved to the corresponding meta-hulks layer.

-   Set new version in the `RUSTVERSION` variable in `poky/meta/conf/distro/include/tcmode-default.inc`
-   Rename files (to new version) in `poky/meta/recipes-devtools/cargo/`
-   Rename files (to new version) in `poky/meta/recipes-devtools/rust/`
-   Some LLVM benchmarks are built and run during the compilation which often results in errors.
    Therefore, it is a good idea to just exclude them by appending `-DLLVM_BUILD_BENCHMARKS=OFF` and `-DLLVM_INCLUDE_BENCHMARKS=OFF` to the `EXTRA_OECMAKE` variable in `poky/meta/recipes-devtools/rust/rust-llvm.inc`.
-   Set new version in the `RS_VERSION` and `CARGO_VERSION` variable in `poky/meta/recipes-devtools/rust/rust-snapshot.inc`
-   Update the checksums in `poky/meta/recipes-devtools/rust/rust-snapshot.inc` for the NAO architecture `x86_64`
    -   Download the files in your command line (example for Rust version 1.63):
    ```sh
    RS_VERSION="1.63.0"
    CARGO_VERSION="1.63.0"
    RUST_BUILD_ARCH="x86_64"
    RUST_STD_SNAPSHOT="rust-std-${RS_VERSION}-${RUST_BUILD_ARCH}-unknown-linux-gnu"
    RUSTC_SNAPSHOT="rustc-${RS_VERSION}-${RUST_BUILD_ARCH}-unknown-linux-gnu"
    CARGO_SNAPSHOT="cargo-${CARGO_VERSION}-${RUST_BUILD_ARCH}-unknown-linux-gnu"
    wget "https://static.rust-lang.org/dist/${RUST_STD_SNAPSHOT}.tar.xz"
    wget "https://static.rust-lang.org/dist/${RUSTC_SNAPSHOT}.tar.xz"
    wget "https://static.rust-lang.org/dist/${CARGO_SNAPSHOT}.tar.xz"
    ```
    -   Generate the checksums in the same terminal:
        ```sh
        sha256sum ${RUST_STD_SNAPSHOT}.tar.xz ${RUSTC_SNAPSHOT}.tar.xz ${CARGO_SNAPSHOT}.tar.xz
        ```
    -   Keep the terminal open for the next step
-   Update the checksums in `poky/meta/recipes-devtools/rust/rust-source.inc`
    -   Download the files:
        ```sh
        wget "https://static.rust-lang.org/dist/rustc-${RS_VERSION}-src.tar.xz"
        ```
    -   Generate the checksums in the same terminal:
        ```sh
        sha256sum "rustc-${RS_VERSION}-src.tar.xz"
        ```
-   Run `bitbake nao-image` within the build shell
    -   Errors similar to `libstd-rs-1.63.0-r0 do_patch: Applying patch...` often mean that patches are obsolete.
        These patches are located in `poky/meta/recipes-devtools/rust/libstd-rs/` and `poky/meta/recipes-devtools/rust/rust-llvm/`.
        Deleted patches need to be removed from their corresponding recipes.
        Afterwards rerun the image build.
-   Once a successful build completed, create a patch from the changes in poky:
    -   ```sh
        cd poky/
        git add .
        git commit # ...
        git format-patch HEAD~  # this generates 0001-....patch
        ```
    -   Copy the patch file into `meta-hulks/patches/0001....patch` and fix the patch path in `meta-hulks/kas-project.yml`
