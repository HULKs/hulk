---
name: review-architecture-fit
description: Use when reviewing PRs or local diffs for wrong crate placement, layer violations, dependency direction issues, node conventions, framework fit, or migration inconsistency.
---

# Review Architecture Fit

## Focus

Find changes that do not fit the repository's crate boundaries, node model, framework conventions, dependency direction, or ongoing migration patterns.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- Shared domain types placed inside a node or tool crate instead of `types`, `framework`, `geometry`, or another shared crate.
- Low-level crates depending on tools, binaries, nodes, or higher-level runtime code.
- New crates missing workspace membership, workspace dependency use, or coherent ownership.
- Node changes that ignore established creation, cycle, parameter, input, output, or runtime conventions.
- Migration changes that use names, topics, parameters, or wiring inconsistent with nearby migrated code.
- Boundary leaks where deployment, config, docs, or tooling concerns enter core logic without a clear reason.

## Severity

- `blocking`: dependency direction, crate placement, or runtime wiring creates a serious architectural or build risk.
- `important`: the change should move to a better crate, layer, or convention before it spreads.
- `suggestion`: naming or placement could align better with adjacent code.

## Output

Report only architecture-fit findings. Cite the changed location and the existing convention it should follow. If nothing matches, write `No findings for architecture fit.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not propose broad architecture rewrites unrelated to the changed scope.
- Do not block deliberate migrations when the diff documents the transitional boundary.
