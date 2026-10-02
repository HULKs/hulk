# Legacy NAO Image and SDK

The Yocto image/toolchain build guide is preserved in
[Historical: NAO Image & SDK](../historical/setup/nao_image_and_sdk.md).
It includes notes about obsolete configuration names and source paths; check
the external `meta-nao` revision you intend to build.

Current Booster deployment uses a K1 AArch64 SDK container, configured by
`sdk_version` in root `hulk.toml`. See
[Development Environment](development_environment.md#build-pepsi-and-install-the-sdk)
and [Booster K1 Setup](booster_setup.md). Current `os_version` identifies the
expected Booster vendor OS, not a NAO Yocto image release.
