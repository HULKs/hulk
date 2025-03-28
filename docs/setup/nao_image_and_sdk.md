# NAO Image & SDK

The HULKs team utilizes the [Yocto Project](https://yoctoproject.org) to create a custom Linux distribution referred to as HULKs-OS.
This framework is not a Linux distribution itself but provides a comprehensive set of tools and processes to build tailored operating systems, especially for embedded devices such as the NAO robot.
Yocto enables the creation of a versatile toolchain that compiles all necessary dependencies, tools, and the Linux kernel to produce flashable OPN image files for the NAO robot.
Alongside the image files, Yocto facilitates the generation of a comprehensive Software Development Kit (SDK), inclusive of a full cross-compilation toolchain designed specifically for the NAO's architecture.

The latest iteration of HULKs-OS is released on GitHub [here](https://github.com/hulks/meta-nao/releases), enabling users to flash images or deploy software onto their robots without constructing their own builds.
When powered up, this custom image automatically configures both wired and wireless network interfaces.
Each NAO robot is distinguished by a unique Head-ID, which is instrumental in assigning a specific IP address.
The mapping between Head-IDs and their corresponding IPs is defined in the `team.toml` file, accessible [here](https://github.com/HULKs/hulk/tree/main/etc/parameters/team.toml), and is uploaded to the NAO during the `pepsi gammaray` deployment process.

For robots not specified in the `team.toml`, the image defaults to configuring its wired network interface via DHCP, allowing users to identify the robot's IP address by checking DHCP leases or consulting with your IT administrator.

## Yocto Project Fundamentals

The [Yocto Project](https://yoctoproject.org) is an industry-standard framework, extensively applied in building custom Linux-based systems for embedded applications.
It manages tasks with [BitBake](https://en.wikipedia.org/wiki/BitBake), a task execution engine responsible for building packages and handling complex dependencies through defined recipes.
Recipes dictate how software should be fetched, compiled, packaged, and installed.

To enhance functionality, Yocto projects utilize [OpenEmbedded](https://www.openembedded.org/wiki/Main_Page), a collection of metadata, recipes, and configuration files providing foundational support to BitBake in building robust packages.
Developers typically use [Poky](https://git.yoctoproject.org/poky/), Yocto's reference distribution, as a prototype to create customized distributions.

Tools like [siemens/kas](https://github.com/siemens/kas) assist in simplifying the setup of Yocto environments with multiple layers and repositories, ensuring consistent and reproducible build processes.

## Creating the Image & SDK

The foundational NAO support for creating a minimal NAO robot operating system for use in SPL is set up using the root `meta` layer in the [`meta-nao`](https://github.com/hulks/meta-nao) repository.
It includes necessary recipes and configurations, automatically releasing updates via continuous integration (CI).
For specific adaptations related to the competition and the HULKs' unique requirements, an additional `meta-hulks` layer is applied to further customize the distribution.

### Setting Up the Working Directory

To get started, clone the code and set up a Yocto working directory.
This working directory will contain the Yocto configuration layers, including [HULKs/meta-nao](https://github.com/HULKs/meta-nao) and `meta-hulks`.

```sh
mkdir yocto
cd yocto
```

!!! danger

    Ensure there is at least 100 GB of free disk space available for creating the image and SDK.

Clone the `meta-nao` repository:

```sh
git clone git@github.com:HULKs/meta-nao
```


We use [siemens/kas](https://github.com/siemens/kas), a setup tool for BitBake-based projects.
To run kas, either install it locally (see instructions [here](https://kas.readthedocs.io/en/latest/userguide/getting-started.html)), or use the containerized version via the [kas-container script](https://github.com/siemens/kas/blob/master/kas-container).
We recommend the containerized method for its convenience and comprehensive package inclusions.
The `kas-container` script facilitates running a container via Podman or Docker.

```sh
wget https://raw.githubusercontent.com/siemens/kas/master/kas-container
chmod u+x kas-container
```

Next, clone all necessary layers specified in our `kas-project.yml`.
This file defines the project structure that `kas` must set up for the Yocto build process.
Kas supports specifying multiple YAML project files, separated by a colon (`:`), allowing us to divide the configuration into multiple components.
To check out the repositories needed for a HULKs-specific image, run:

```sh
./kas-container checkout meta-nao/kas/base.yml:meta-nao/kas/hulks.yml
```

The final step involves populating the working directory with the proprietary and closed-source software by Aldebaran, notably LoLA and HAL for communication with the chestboard.
We do not provide these binaries; instead, they are extracted from the `.opn` files delivered with the RoboCupper image.
HULKs members can request the RoboCupper image from our dev leads, whereas non-members should contact the RoboCup SPL Technical Committee.

To extract the required binaries, we provide a script named `extract_binaries.sh`.
This script mounts the file system contained in the OPN image, retrieves all binaries from the RoboCupper image, and archives them for the subsequent build phase.
Note that mounting the OPN file system might require root privileges.

```sh
cd meta-nao/meta/recipes-support/aldebaran/
mkdir -p aldebaran-binaries
./extract_binaries.sh -o aldebaran-binaries/aldebaran_binaries.tar.gz nao-2.8.5.11_ROBOCUP_ONLY_with_root.opn
```

Your working directory is now ready to build your own NAO image and SDK.
You may customize the distribution to suit your needs, such as installing additional dependencies, configuring the kernel, and more.

### Initiating a Build Shell

`kas` can start a shell within the build environment.
Reference the `kas-project.yml` of meta-nao:

```sh
# working directory is `yocto`
./kas-container shell meta-nao/kas/base.yml:meta-nao/kas/hulks.yml
```

All BitBake and Devtool commands should be executed from within this shell.

!!! hint

    Your `TERM` variable may request a terminfo that is not available in the kas container image. This results in e.g. progress bars not displaying correctly. Make sure, your terminfo is supported within the container. You may be able to just override it with xterm:
    ```sh
    export TERM=xterm-256color
    ```

### Building the Image

Within the build shell, you can build a NAO OPN image and SDK using BitBake.
The initial build may take several hours, depending on your computer's performance and download speed.
Keep in mind that you're building an entire Linux distribution.
BitBake provides advanced caching of build artifacts, which significantly reduces the duration of future builds to minutes or even seconds, depending on the changes made.
The cache resides in `build/sstate-cache`, which can be copied from another build directory or shared between machines.
For more information, refer to the [Yocto Documentation on Shared State Cache](https://docs.yoctoproject.org/overview-manual/concepts.html#shared-state-cache).
To build the image, run the following command from within the build shell:

```sh
bitbake nao-image
```

This command generates and executes all necessary tasks and targets to create a proper `.opn` file.
Once BitBake completes the `nao-image` task, the image file will be located at `build/tmp/deploy/images/nao-v6/nao-image-HULKs-OS-[...].ext3.gz.opn`.
The image can be directly flashed to a NAO as detailed in the [NAO setup](./nao_setup.md#flashing-the-firmware) section.

### Building the SDK

To compile software targeting the NAO platform, the code needs to be cross-compiled for the NAO target.
When you only change configuration for the NAO image, you may still maintain compatibility with the publicly released SDK at [meta-nao](https://github.com/hulks/meta-nao/releases) and opt for this SDK instead of building your own.

![Cross Canadian Compilation](./cross-canadian-light.svg#only-light)
![Cross Canadian Compilation](./cross-canadian-dark.svg#only-dark)
/// caption
Cross Canadian Compilation
///

Within the build shell, execute the following command to build a complete SDK:

```sh
bitbake -c populate_sdk nao-image
```

Again, this process might take several hours.
After a successful build, the SDK will be located at `build/tmp/deploy/sdk/HULKs-OS-toolchain-[...].sh`.
To install the SDK, run the script and follow the instructions.
Once installed, you can source the build environment and use the respective cross-compilers.

### Advanced Topics

#### Updating Yocto Layers

The Yocto Project and the Poky reference distribution provide a Linux kernel, userland programs, libraries, and other tooling, which are regularly updated in Yocto releases.
To ensure deterministic builds, the HULKs freeze the versions of all used layers in the kas project files of meta-nao (`base.yml`, `hulks.yml`, ...).

#### Versioning of Image/SDKs Using Semantic Versioning

The HULKs adhere to semantic versioning for Yocto images and SDKs.
This versioning system increases version numbers based on the nature of the changes:

- Both images and SDKs have major, minor, and patch version numbers (e.g., 4.2.3).
- Images and SDKs with the same major and minor version numbers are compatible with each other.
- Major changes, refactorings, or implementations necessitate an increase in the major version number.
- Minor changes, additions, and iterations necessitate an increase in the minor version number.
- Changes in the image that do not require SDK recreation result in an increase in the patch version number. Consequently, only a new image needs to be created, not necessarily a redistribution of new SDKs.

Before building new images, the version number must be set in `meta-nao/meta-hulks/conf/distro/HULKs-OS.conf`.
Only modify the `DISTROVERSION`; the `SDKVERSION` is automatically derived from the `DISTRO_VERSION`.

After releasing a new image and/or SDK, update the `OSVERSION` and/or `SDKVERSION` variables in `crates/constants/src/lib.rs`.
Successive builds with pepsi will use the new version.

#### Upgrade Rust Version

Since upgrading the Rust version often requires manual steps, this section describes the approach on how to upgrade and generate the needed patch files.
These instructions can be followed e.g. if a new Rust version is available and a new image/SDK should be created with this new version.
Users that just want to use the current version that we upgraded to should skip this section.

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
