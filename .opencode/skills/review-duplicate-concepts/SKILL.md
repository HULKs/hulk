---
name: review-duplicate-concepts
description: Use when reviewing PRs or local diffs for duplicated concepts, reimplemented helpers, missed reuse, or parallel abstractions.
---

# Review Duplicate Concepts

## Focus

Find added code that reimplements an existing concept instead of reusing, extending, or moving the existing implementation.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- New helpers, structs, traits, algorithms, parameters, or messages that duplicate existing names or behavior.
- Local implementations of concepts that already live in shared crates such as `geometry`, `filtering`, `framework`, `types`, `parameters`, or `ros-z*`.
- Copy-pasted logic with small naming or type changes.
- New abstractions that overlap with existing abstractions but do not replace them.
- Missed extension points where the existing implementation should grow instead of adding a parallel path.

## Severity

- `blocking`: duplicate behavior will likely diverge, breaks a single source of truth, or conflicts with an existing public abstraction.
- `important`: reuse is straightforward and would reduce maintenance or improve consistency.
- `suggestion`: similar concept exists, but specialization may be justified.

## Output

Report findings only for duplicate or missed-reuse issues. Include evidence from both the changed code and the existing implementation. If nothing matches, write `No findings for duplicate concepts.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not flag intentional specialization when the diff clearly explains why reuse is wrong.
- Do not require large refactors when a small call into existing code solves the issue.
