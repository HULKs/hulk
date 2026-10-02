!!! info "Historical NAO operating system"

    This page preserves the NAO/Yocto operating-system overview. It does not describe the current Booster K1 deployment. See [current operating system](../../operating_system/overview.md). The team-number note below is historical; check the selected image's network configuration and remaining hardcoded routes.

The HULKs use the [Yocto Project](https://yoctoproject.org) for creating a custom linux distribution, we call HULKs-OS.
The toolchain compiles all necessary dependencies, tools, and kernel to produce flashable OPN images for the NAO.
Additionally, Yocto provides means to construct a corresponding software development kit (SDK) containing a complete cross-compilation toolchain.

Team HULKs automatically releases the latest HULKs-OS publicly on GitHub [here](https://github.com/hulks/meta-nao/releases).
If you're looking to use these images or SDKs for flashing and deploying software onto your robot, you can opt for the pre-built versions and do not need to build your own image and SDK.

!!! info

    Currently the team number is hardcoded into the image.
    To change it, several files need to be modified.
