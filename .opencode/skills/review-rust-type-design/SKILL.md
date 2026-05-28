---
name: review-rust-type-design
description: Use when reviewing Rust diffs for weak type modeling, stringly typed states, boolean-heavy invariants, magic sentinels, or missed enums and newtypes.
---

# Review Rust Type Design

## Focus

Find Rust changes where the type system should carry invariants that are currently enforced by comments, strings, booleans, sentinels, or scattered runtime checks.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- Finite states represented as `String`, integers, or unrelated booleans instead of enums.
- Invalid combinations that could be impossible with a more precise type.
- IDs, units, coordinates, durations, and thresholds that need newtypes or existing domain types.
- Sentinel values where `Option`, `Result`, `NonZero*`, ranges, or enums would be clearer.
- Public structs that expose fields too loosely for downstream callers to use safely.
- Serialization changes that need explicit names, defaults, or `deny_unknown_fields` for configuration-like data.

## Severity

- `blocking`: the weak type model permits invalid robot, message, or API states with realistic failure impact.
- `important`: a precise enum, newtype, or option would prevent common misuse or remove repeated runtime validation of the same invariant.
- `suggestion`: type clarity could improve readability but current risk is low.

## Output

Report only type-design findings. Explain the invariant and name a compact Rust type shape that would encode it. If nothing matches, write `No findings for Rust type design.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not propose typestate or complex generics for simple local logic.
- Do not demand newtypes where existing domain types already communicate the invariant clearly.
