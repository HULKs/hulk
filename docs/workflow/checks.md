# Checking your code

We use a number of tools to automatically check our code for common mistakes.
These checks are automatically executed by GitHub when you submit new code,
but you can also run them locally to check your code before submitting.

These are the most important checks:

| Check                 | How to run                                               |
|-----------------------|----------------------------------------------------------|
| Run unit tests        | `cargo test --workspace`                                 |
| Lint Rust code        | `./pepsi clippy --workspace`                             |
| Format Rust code      | `cargo fmt`                                              |
| Format TOML files     | Install [Taplo](https://taplo.tamasfe.dev/); `taplo fmt` |
| Check parameter files | `cargo run --bin parameter_tester`                       |

The checks performed in our CI workflow are defined in `.github/workflows/pull-request.yml`.

