---
name: review-semantic-usefulness
description: Use when reviewing PRs or local diffs for vacuous changes, unreachable features, incomplete behavior, unused code paths, or mechanical edits without user value.
---

# Review Semantic Usefulness

## Focus

Find changes that appear implemented but do not produce useful, reachable, or complete behavior for callers, tools, robots, or maintainers.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- New helpers, types, parameters, or features that no production path uses.
- Tests that verify isolated code while the feature remains unreachable.
- Mechanical rewrites that do not change behavior, readability, safety, or maintainability.
- Partially implemented features with no clear guard, error, documentation, or follow-up boundary.
- Code that computes values but drops them, logs them only, or never exposes them to the intended consumer.
- Claimed behavior in docs or PR text that the diff does not actually implement.

## Severity

- `blocking`: the PR claims functionality that is unreachable, incomplete, or misleading.
- `important`: meaningful behavior is only partially wired or not validated end to end.
- `suggestion`: the change could better demonstrate its value with a call site, example, or clearer boundary.

## Output

Report only semantic usefulness findings. State the claimed or implied behavior and the missing path that makes it useful. If nothing matches, write `No findings for semantic usefulness.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not judge product priority without evidence.
- Do not reject preparatory work when the diff clearly documents the boundary and the work is reviewable on its own.
