---
name: review-atomicity-and-splitting
description: Use when reviewing PRs or local diffs for oversized changes, mixed concerns, unclear commit boundaries, unrelated features, or changes that should be split.
---

# Review Atomicity And Splitting

## Focus

Find changes that would be easier, safer, and faster to review as smaller commits or PRs with clear boundaries.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- Multiple unrelated features in one branch.
- Refactors mixed with behavior changes and formatting churn.
- Generated files mixed with hand-written logic without a separate boundary.
- Tests or docs covering only one of several behavior changes.
- Commit history that makes review harder because each commit is not independently understandable.
- A large migration that could be split by crate, node, API surface, or compatibility layer.

## Severity

- `blocking`: the change is too broad to review safely or hides risky behavior behind unrelated churn.
- `important`: splitting would materially improve review quality and reduce rework.
- `suggestion`: commit grouping or PR description could clarify the review path.

## Output

Report only atomicity and splitting findings. Suggest concrete split boundaries such as crates, features, generated output, refactor-first, behavior-second, docs-third, or migration slices. If nothing matches, write `No findings for atomicity and splitting.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not demand splitting for a cohesive cross-cutting change when the coupling is clear.
- Do not block on size alone; explain the independent concerns or review risks.
