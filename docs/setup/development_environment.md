# Development Environment

This page covers development for Booster K1 and the current ROS-Z stack.

## Rust and Host Tools

Linux is the primary development environment. Install Rust through
[rustup](https://rustup.rs/). Use the workspace's required Rust version, currently
`1.98.0` in `Cargo.toml`. The SDK uses 1.98.0; pull-request CI uses 1.98.1.
The documentation workflow still uses the 1.98.0 CI image.
Install the `clippy` and `rustfmt` components for local checks.

For native builds and deployment, install Git, Git LFS, a C/C++ toolchain,
Clang/libclang, CMake, pkg-config, OpenSSL development files, ALSA development
files, Python 3, rsync, OpenSSH, and `ping`. Native GUI/simulator builds also
need a working graphics environment and may need udev development files.
Package names vary by distribution; see `flake.nix` and
`tools/ci/github-runners/Containerfile` for the repository's environment definitions.
This is a task-oriented prerequisite list, not a tested package recipe for every distribution.

Install **Podman** for the default cross-compilation workflow. Pepsi also accepts
`--env docker`, but `sdk install`, `sdk build`, and `sdk list` use Podman.
Using Docker requires an appropriately tagged SDK image in Docker's image store;
installing Docker alone does not prepare it or change Pepsi's default environment.
Native non-Linux development has not been validated by these instructions.

On x86_64 Linux, an alternative development environment is:

```sh
nix develop
```

The flake provides the toolchain and libraries for its supported tools. Podman
and the [check-specific prerequisites](../workflow/checks.md) still need to be
available for the commands you intend to run.

## Clone the Repository

Configure your Git name, email, and GitHub authentication before contributing.
Then run:

```sh
git lfs install
git clone https://github.com/HULKs/hulk.git
cd hulk
git lfs pull
```

Neural-network models and other assets are stored with Git LFS. If model loading
fails, check that LFS downloaded the actual files rather than leaving pointer
files. `git lfs ls-files` lists managed assets; `git lfs pull` retrieves missing ones.

## Build Pepsi and Install the SDK

Run commands from the repository root:

```sh
./pepsi --help
./pepsi sdk install
```

The launcher builds **Pepsi and its dependencies**, then executes it. Other
workspace tools and the robotics executable are built when requested.

`sdk install` explicitly pulls `ghcr.io/hulks/k1sdk:<sdk_version>` using Podman.
The version comes from root `hulk.toml` (currently `1.4.0`).

When a Podman cross-build finds no locally tagged SDK image, Pepsi instead
**builds** one from `tools/sdk_container/`. Remote Podman builds similarly run
`sdk build` on the remote host if needed. Container execution uses `--pull=never`;
it does not automatically download the latest published image.

```sh
./pepsi sdk list
./pepsi sdk build
./pepsi sdk install --help
```

Native builds use `target` by default; SDK builds use `target/container`.
The SDK targets `aarch64-unknown-linux-gnu`. See
[Pepsi build environments and directories](../tooling/pepsi.md#build-environments-and-directories)
for overrides, remote builds, and artifact locations.

For a robot cross-build, select the robotics manifest so Pepsi sees its
`cross-compile` metadata:

```sh
./pepsi build crates/hulk_ros_z
```

You can install tools for convenient use without the launcher:

```sh
./pepsi install pepsi
./pepsi install twix
```

The default install directory is `~/.cargo/bin`. Add it to your `PATH` and
reinstall when you need updates. Examples in this section use `./pepsi` to build
and run the version in the checkout.

## Local Simulator and Debugging

The workspace includes the [behavior tree simulator](../tooling/behavior_tree_simulator_design.md):

```sh
cargo run -p bevyhavior_simulator --bin behavior_tree_smoke
```

For the separate experimental MuJoCo simulator, use the development checkout
linked in the [simulator guide](../tooling/behavior_simulator.md).

[Twix](../tooling/twix.md) uses ROS-Z namespaces and an optional router endpoint,
for example:

```sh
./pepsi run twix -- /42 --router tcp/10.1.24.42:7447
```

## Inference Runtime Compatibility

The robot runtime Containerfile uses an ONNX Runtime 1.22 / CUDA 12.8 base image.
The existing runtime is documented as using TensorRT 10.7 and cuDNN 9.7; confirm
the installed image's contents when changing it. The Rust workspace uses `ort`
rc.13 with `api-21`. Do not enable its default features or `api-22` and newer
without revalidating the runtime. Automatic device selection enabled by `api-22`
aborted in the ONNX Runtime 1.22 CPU build during local validation.

Use `GraphOptimizationLevel::All` to retain rc.10's `Level3` behavior.
In rc.13, `Level3` selects `ORT_ENABLE_LAYOUT`, which ONNX Runtime 1.22
rejects with `graph_optimization_level is not valid`.

`tools/k1-setup/hulk-runtime.container` sets both `ORT_DYLIB_PATH` and
`LD_PRELOAD` to `/usr/local/lib/libonnxruntime.so`. Preloading addresses the rc.13
shutdown order: its environment must be released before ONNX Runtime's C++
destructors. Loading only through `ORT_DYLIB_PATH` produced a heap-corruption
abort on process exit with the Linux x64 ONNX Runtime 1.22 build, even with
only a session builder and no inference.

`pepsi gammaray` uploads the container definition before reloading systemd,
restarts `hulk-runtime.service` to recreate the container, and then ensures
HULK is started. Systemd stops HULK before its runtime through the service's
`Requires=` and `After=` dependencies. Run this setup before deploying the
upgraded binaries.

This does not require a new image or a CUDA upgrade. Verify startup and
shutdown on the robot, including GPU provider selection and model inference,
before rollout. Local CPU validation does not cover Jetson CUDA or TensorRT
execution. For local dynamic-loading runs, set both variables to the same
absolute library path.

## Documentation Preview

From the repository root, preview the documentation with uv:

```sh
uvx --with mkdocs-material mkdocs serve
```

Open `http://127.0.0.1:8000`; the preview reloads when pages change.
Build the static site with the strict check used by CI:

```sh
uvx --with mkdocs-material mkdocs build --strict
```

The generated site is written to `site/`. Page sources are in `docs/`, and navigation is configured in `mkdocs.yml`.
