---
name: review-rust-ownership
description: Use when reviewing Rust diffs for ownership-sensitive API design, function signatures, unnecessary clones, avoidable allocations, inefficient borrowing, or hot-path data movement.
---

# Review Rust Ownership

## Focus

Find Rust code and newly introduced API surfaces that fight ownership, allocate unnecessarily, or move data in ways that make code slower or harder to read.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- Unnecessary `.clone()`, `.to_owned()`, `.to_string()`, or intermediate `.collect()` calls.
- New or changed function signatures, public APIs, traits, builders, and message boundaries where ownership choices should be designed deliberately.
- Parameters that should borrow as `&T`, `&str`, or `&[T]` instead of taking owned values, or APIs that should accept owned values because they store, spawn, or transfer ownership.
- Return types that unnecessarily allocate or clone instead of returning references, iterators, `Cow`, or existing borrowed views when those fit the API contract.
- Cloning or allocation inside loops, cyclers, message paths, stream processing, or runtime hot paths.
- Data structures that force ownership where borrowing, iteration, or existing domain types would be clearer.
- Ownership workarounds that hide a simpler API boundary.
- Ownership design issues that Clippy will not catch, especially whether a new API should expose borrowing, ownership transfer, iteration, or cloning at its boundary.

## Severity

- `blocking`: avoidable hot-path allocation or cloning creates clear runtime risk.
- `important`: ownership choices are likely to hurt maintainability, readability, or performance.
- `suggestion`: a smaller borrow or simpler data flow would improve readability.

## Output

Report only Rust ownership findings. Include why the current ownership behavior matters at the cited call site or API boundary. If nothing matches, write `No findings for Rust ownership.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not flag clones that are required for ownership, concurrency, lifetime boundaries, or API contracts.
- Do not demand lifetime gymnastics that make the code less readable without a concrete benefit.
- Do not optimize allocations without evidence from the changed path, nearby usage, or the ownership contract introduced by a new API signature.
