---
name: review-rust-error-handling
description: Use when reviewing Rust diffs for panic paths, unwrap or expect usage, weak Result handling, dropped error sources, poor error context, or recoverability mistakes.
---

# Review Rust Error Handling

## Focus

Find Rust code that panics in production paths, weakens error information, or handles recoverable failures in a way that hurts diagnostics or reliability.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- `unwrap()`, `expect()`, `panic!()`, and indexing that can panic outside tests or proven impossible states.
- `Result` handling that drops sources, hides context, or converts recoverable errors into panics.
- Expected missing data or transient runtime failures treated as fatal without justification.
- Error messages that omit the operation, path, parameter, topic, robot, or value needed to debug the failure.
- Error choices that match the crate role: contextual reports for binaries and tools, typed errors where library callers need to react.

## Severity

- `blocking`: production panic, lost critical error source, or misleading error behavior creates clear runtime or diagnostic risk.
- `important`: error handling is likely to hurt reliability, recoverability, or debugging.
- `suggestion`: clearer context or a smaller `Result` flow would improve readability.

## Output

Report only Rust error-handling findings. Include why the current failure behavior matters at the cited call site. If nothing matches, write `No findings for Rust error handling.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not ban all panics in tests, build scripts, or proven impossible internal invariants.
- Do not require a custom error enum where contextual propagation is enough for the crate role.
- Do not ask for elaborate recovery paths when failing fast is documented and appropriate.
