# Parameter Tester

The parameter tester is a tool to test if the `default.json` parameters can be correctly parsed and could be loaded on the robot.
It is part of our default tests in the CI pipeline on GitHub.

It can be called with the `pepsi` command or directly using cargo:

```bash
./pepsi run parameter_tester
```

```bash
cargo run --bin parameter_tester
```
