# Rust Error Handling

## Focus

Find Rust code that panics in production paths, weakens error information, or handles recoverable failures in a way that hurts diagnostics or reliability.

## What To Check

- `unwrap()`, `expect()`, `panic!()`, and indexing that can panic outside tests or proven impossible states.
- `Result` handling that drops sources, hides context, or converts recoverable errors into panics.
- Expected missing data or transient runtime failures treated as fatal without justification.
- Error messages that omit the operation, path, parameter, topic, robot, or value needed to debug the failure.
- Error choices that match the crate role: contextual reports for binaries and tools, typed errors where library callers need to react.

## Severity Guidance

- `blocking`: production panic, lost critical error source, or misleading error behavior creates clear runtime or diagnostic risk.
- `important`: error handling is likely to hurt reliability, recoverability, or debugging.
- `suggestion`: clearer context or a smaller `Result` flow would improve readability.

## Output Guidance

Report only Rust error-handling findings. Include why the current failure behavior matters at the cited call site. If nothing matches, write `No findings for Rust error handling.`

## Criterion-Specific Do Not

- Do not ban all panics in tests, build scripts, or proven impossible internal invariants.
- Do not require a custom error enum where contextual propagation is enough for the crate role.
- Do not ask for elaborate recovery paths when failing fast is documented and appropriate.
