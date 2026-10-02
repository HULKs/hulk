# Checking your code

We use a number of tools to automatically check our code for common mistakes.
These checks are automatically executed by GitHub when you submit new code,
but you can also run them locally to check your code before submitting.

## Local Checks Matching CI

Run from the repository root. The active jobs in
`.github/workflows/pull-request.yml` use these commands:

| Check | Command |
| --- | --- |
| Rust linting, with warnings denied | `./pepsi clippy --locked . -- --deny warnings` |
| Cargo.lock consistency | `cargo update --locked --workspace --manifest-path Cargo.toml` |
| Rust, TOML, and Python formatting (check only) | `./pepsi fmt --check` |
| Unit and integration tests through Nextest | `./pepsi nextest` |
| Documentation tests | `./pepsi test --doc` |
| Release build of Pepsi | `./pepsi build --locked --release pepsi` |
| Release build of Twix | `./pepsi build --locked --release twix` |
| Documentation build | `mkdocs build --strict` |

CI runs on pull requests and merge-group check requests. Its Rust jobs use
`ghcr.io/hulks/hulk-ci:1.98.0`; native local checks need the corresponding
toolchain and [development dependencies](../setup/development_environment.md).
The table lists the active checks, not the commented-out workflow jobs.

## Check-Specific Prerequisites

- Install the Rust `clippy` and `rustfmt` components through rustup.
- Install [Taplo CLI](https://taplo.tamasfe.dev/) for TOML formatting.
- Install [uv](https://docs.astral.sh/uv/) so the formatter can run
  `uvx ruff format` on tracked Python files. CI currently installs uv `0.10.9`.
- Install [cargo-nextest](https://nexte.st/) for `./pepsi nextest`.
  CI installs it using cargo-binstall.
- Install `mkdocs-material` for the strict documentation build, as the CI job does.
- Fetch Git LFS assets; CI checks out with `lfs: true`.

`./pepsi test` is still useful for running Cargo tests locally, but CI uses
Nextest and runs doctests separately. Plain `./pepsi clippy` does not deny
warnings like the CI invocation does.

## Apply Formatting

`./pepsi fmt` applies Rust, TOML, and Python formatting and **edits files**.
Use `--check` for verification without edits. `cargo fmt` and `taplo fmt` remain
useful for individual formats, but do not cover the complete formatter workflow.

## Robot Validation

Explicit robot-target and service build jobs are currently commented out in CI.
Green CI therefore does not by itself establish a successful robot cross-build
or working on-robot inference. For changes to the robot stack, build the selected
robot manifest with `./pepsi build crates/hulk_ros_z` and test relevant behavior
on the simulator/robot. See [upload verification](../setup/upload.md#check-startup-and-logs).
