# NAO Image & SDK

The HULKs use the [Yocto Project](https://yoctoproject.org) for creating a custom linux distribution, we call HULKs-OS.
The toolchain compiles all necessary dependencies, tools, and kernel to produce flashable OPN images for the NAO.
Additionally, Yocto provides means to construct a corresponding software development kit (SDK) containing a complete cross-compilation toolchain.

Team HULKs automatically releases the latest HULKs-OS publically on GitHub [here](https://github.com/hulks/meta-nao/releases).
If you're looking to use these images or SDKs for flashing and deploying software onto your robot, you can opt for the pre-built versions and do not need to build your own image and SDK.

Upon booting, the image automatically configures both wired and wireless network devices for the NAO.
Each robot is identified by its unique Head-ID, which is used to assign a distinct IP address.
The mapping between Head-IDs and IP addresses is configured in the image ([here](https://github.com/HULKs/meta-nao/blob/main/meta-hulks/recipes-hulks/network-config/network-config/id_map.json)).
All HULKs robots come pre-configured in the released images, via the `configure_network` service ([here](https://github.com/HULKs/meta-nao/blob/main/meta-hulks/recipes-hulks/network-config/network-config/configure_network)).
But if you're flashing a new or non-HULKs robot, you'll need to add its head ID to the map and generate a new image.

For robots not listed, the image falls back to configuring its wired network device via DHCP.
Thus, you're free to flash the HULKs-OS image onto a robot and find its IP address by inspecting your DHCP leases.

## Image & SDK Creation

The Yocto Project leverages [BitBake](https://en.wikipedia.org/wiki/BitBake) as task execution engine and offers an abstraction layer to modify and extend existing build configurations.
Combined with [OpenEmbedded](https://www.openembedded.org/wiki/Main_Page), the entire worktree is structured in several layers configuring the distribution and providing support for dependencies or services.
Basic NAO support to construct a distribution for a minimal NAO robot operating system is configured with the root `meta`-layer in the [`meta-nao`](https://github.com/hulks/meta-nao) repository.
The HULKs overlay this configuration with an additional `meta-hulks` layer to target the special SPL and HULKs usecase.

### Setup of the Working Directory


Start by cloning the code and setting up a Yocto working directory.
This working directory will contain the yocto configuration layers, including [HULKs/meta-nao](https://github.com/HULKs/meta-nao) and meta-hulks.
Additionally, a helper script for running BitBake commands.
It will be called `yocto`.
The new `yocto` working directory must have at least 100 GB of empty space available and should not be part of a Git repository.

```sh
mkdir yocto/
cd yocto/
```

!!! danger

    For creating the image and SDK, make sure there is at least 100 GB empty disk space available.


Continue with cloning the `meta-nao` repository:

```sh
git clone git@github.com:HULKs/meta-nao
```

For project setup we use [siemens/kas](https://github.com/siemens/kas), a setup tool for bitbake based projects.
To run kas, either install it locally ([see here](https://kas.readthedocs.io/en/latest/userguide/getting-started.html)), or use the containerized version via the [kas-container script](https://github.com/siemens/kas/blob/master/kas-container).
We prefer the containarized solution, as the container comes with all batteries included.
The `kas-container` script makes it easy to spin a container (via podman or docker).

```sh
wget https://raw.githubusercontent.com/siemens/kas/master/kas-container
chmod u+x kas-container
```

Subsequently, you can clone all necessary layers, we specify in our `kas-project.yml`.
This file defines the project structure `kas` has to setup for the Yocto build phase.

```sh
./kas-container checkout meta-hulks/kas-project.yml
```

The last step is to populate the working directory with the proprietary and not open source released software by aldebaran.
This mainly is LoLA and HAL for communication with the chestboard.
We do **not** provide these binaries, but rather extract them from the `.opn` files shipped with the RoboCupper image.
For HULKs members contact our dev-leads and for non HULKs members contact the RoboCup SPL Technical Committee to get this image.

To extract the necessary binaries we provide a helper script called `extract_binaries.sh`.
This script mounts the file system contained in the OPN image, fetches all binaries from inside the RoboCupper image, and collects them in an archive for the upcoming build phase.
Mounting the OPN file system may require root privileges.

```sh
cd meta-nao/recipes-support/aldebaran/
mkdir -p aldebaran-binaries
./extract_binaries.sh -o aldebaran-binaries/aldebaran_binaries.tar.gz nao-2.8.5.11_ROBOCUP_ONLY_with_root.opn
```

Now, your working directory is ready to build your own NAO image and SDK.
At this point, you may adjust the distribution to your liking.
This includes adding hardware IDs, configuring network, installing additional dependencies, and much more.

!!! todo

    Explain what to do when configuring a new robot.

### Starting a Build Shell

`kas` is able to start a shell inside of the build environment.
The `kas-project.yml` of meta-nao needs to be referenced:

```sh
# working directory is `yocto`
./kas-container shell meta-nao/kas-project.yml
```

All BitBake and Devtool commands must be executed from inside this shell.

### Building the Image

Inside of the build shell, you can build a NAO OPN image or SDK via BitBake.
The initial build may take multiple hours depending on your computing performance and internet downlink speed.
Remember, you are building an entire linux distribution.
BitBake provides advanced caching of the build artifacts which means that future builds are done in minutes or even seconds depending on the changes.
The cache relies in the `build/sstate-cache` which can be copied from another build directory or even shared between machines, see [Yocto Documentation about Shared State Cache](https://docs.yoctoproject.org/overview-manual/concepts.html#shared-state-cache) for further explanation.
To build the image, run the following command from inside the build shell:

```sh
bitbake nao-image
```

This generates and executes all necessary tasks and targets to construct a proper `.opn` file.
As soon as the build has successfully finished, the image is ready to be flashed to a robot.
After BitBake ran all tasks up to `nao-image`, a new `.opn` file is generated in `build/tmp/deploy/images/nao-v6/nao-image-HULKs-OS-[...].ext3.gz.opn`.
The image can now be flashed to a NAO as described in the [NAO setup section](./nao_setup.md#flashing-the-firmware).

### Building the SDK

To be able to compile software targeting the NAO platform, the code needs to be cross compiled for the NAO target.
When you only change configuration for the NAO image, you may still maintain compatability with the publically released SDK at [meta-nao](https://github.com/hulks/meta-nao/releases) and opt for this SDK instead of building your own.

Within the build shell, the following command will build a full SDK:

```sh
bitbake -c populate_sdk nao-image
```

Again, this build phase may take several hours.
After a successful build, the SDK is located at `build/tmp/deploy/sdk/HULKs-OS-toolchain-[...].sh`.
To install the SDK run the script and follow the instructions.
Afterwards, you are able to source the build environment and use the respective cross compilers.

### Advanced:

#### Upgrade other Yocto Layers

The Yocto Project and the Poky reference distribution provide a Linux kernel, userland programs, libraries, and other tooling.
All these things are updated in the regular Yocto releases.
To ensure deterministic builds the HULKs freeze versions of all used layers in the `kas-project.yml` files of meta-nao.

#### Upgrade Image/SDK Versions and Semantic Versioning

The HULKs use semantic versioning for the Yocto images and SDKs.
This means that versions are increased depending on the severity of changes.
The following policy exists for the HULKs:

-   Both images and SDKs have major, minor, and patch version numbers (e.g. 4.2.3).
-   Images and SDKs with the same major and minor version number are compatible with each other.
-   Major changes, refactorings, or implementations result in the increase of the major version number.
-   Minor changes, additionsk, and iterations result in the increase of the minor version number.
-   Changes in the image that do not require SDK recreation, result in the increase of the patch version number. This consequently only requires to create a new image and not necessarily a redistribution of new SDKs.

Before building new images, the version number needs to be set in `meta-nao/conf/distro/HULKsOS.conf`.
Only change the `DISTRO_VERSION`, the `SDK_VERSION` is automatically derived from the `DISTRO_VERSION`.

Once a new image and/or SDK is released, pepsi needs to know the new version numbers.
Therefore update the variables `OS_VERSION` and/or `SDK_VERSION` in `crates/constants/src/lib.rs`.
Successive builds with pepsi will use the new version.

#### Upgrade Rust Version

Since upgrading the Rust version often requires manual steps, this section describes the approach on how to upgrade and generate the needed patch files.
These instructions can be followed e.g. if a new Rust version is available and a new image/SDK should be created with this new version.
Users that just want to use the current version that we upgraded to should skip this section.
The latest patch set for is included in the meta-nao layer (in `patches/`).

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
    -   Copy the patch file into `meta-nao/patches/0001....patch` and fix the patch path in `meta-nao/kas-project.yml`
