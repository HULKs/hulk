# Repository Guidelines

## Project Structure & Module Organization

HULK is a Rust workspace for robot control software and tools. Core crates live in `crates/`, with node implementations under `crates/nodes/*`. CLI and desktop tools live in `tools/`, independent services in `services/`, runtime assets in `etc/`, Webots files in `webots/`, and docs in `docs/`. Rust integration tests usually live in each crate's `tests/` directory; Python tests use `test_*.py` near their package.

## Build, Test, and Development Commands

- `pepsi nextest --remote`: run workspace Rust tests on the remote workstation.
- `pepsi clippy --remote`: lint Rust code remotely.
- `cargo fmt`: format Rust code.
- `pepsi format --check`: verify project formatting as CI does.
- `pepsi run --remote parameter_tester`: validate parameter files in `etc/parameters`.
- `pepsi build --remote --release twix`: build a selected tool; replace `twix` with another target such as `pepsi`.
- `mkdocs build --strict`: build documentation locally.

Use installed `pepsi` when `pepsi --version` matches `./pepsi --version`; for remote-capable commands, put `--remote` after the `pepsi` subcommand so they run on the remote workstation. `pepsi format` has no remote mode. CI checks are in `.github/workflows/pull-request.yml`.

## Coding Style & Naming Conventions

Rust uses edition 2024 and the workspace MSRV in `clippy.toml` (`1.91.1`). Use standard Rust naming: `snake_case` modules, functions, and variables; `CamelCase` types and traits; `SCREAMING_SNAKE_CASE` constants. Python linting is configured by `ruff.toml` with an 80-column line length. `.editorconfig` sets 2-space JSON/XML indentation and 4-space Markdown indentation under `docs/`.

## Testing Guidelines

Add focused tests close to the affected crate or package. For Rust, prefer unit tests beside implementation for narrow logic and integration tests in `tests/` for public behavior. For Python tooling, name tests `test_*.py` and run `uv run pytest` in the relevant package. Run `pepsi nextest --remote` for Rust changes and `pepsi run --remote parameter_tester` when touching parameters.

## Commit & Pull Request Guidelines

Recent history uses short imperative subjects, sometimes with prefixes such as `feat:`. Examples: `Port line detection and intermediate nodes`, `feat: reject ambiguous schema`, `Fix Time Message schema impl`. PRs should follow `.github/pull_request_template.md`: explain why and what changed, link issues, list known issues, note follow-up ideas, and provide reviewer-focused test steps.

## Agent-Specific Instructions

Do not commit generated binaries, simulator captures, or large neural network artifacts unless required. Avoid unrelated files; this repository contains many generated and asset-heavy paths. For `pepsi`, prefer the matching installed binary and use `--remote` on commands that support it.
