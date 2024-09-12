The HULKs use the [Yocto Project](https://yoctoproject.org) for creating a custom linux distribution, we call HULKs-OS.
The toolchain compiles all necessary dependencies, tools, and kernel to produce flashable OPN images for the NAO.
Additionally, Yocto provides means to construct a corresponding software development kit (SDK) containing a complete cross-compilation toolchain.

Team HULKs automatically releases the latest HULKs-OS publicly on GitHub [here](https://github.com/hulks/meta-nao/releases).
If you're looking to use these images or SDKs for flashing and deploying software onto your robot, you can opt for the pre-built versions and do not need to build your own image and SDK.
