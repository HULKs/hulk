---
name: review-tests-behavioral-coverage
description: Use when reviewing PRs or local diffs for missing behavioral tests, weak edge cases, brittle implementation assertions, or untested error paths.
---

# Review Tests Behavioral Coverage

## Focus

Find changed behavior that lacks meaningful tests, and find tests that lock implementation details instead of user-visible or caller-visible behavior.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- New behavior without a test or explicit reason testing is infeasible.
- Bug fixes without a regression test that would have failed before the fix.
- Tests that assert private helper names, file layout, call counts, or migration progress instead of behavior.
- Missing edge cases for empty input, invalid input, boundary values, ordering, timing, and error paths.
- Tests that are nondeterministic, over-mocked, or tightly coupled to incidental implementation.
- PR testing notes that do not explain how the reviewer can validate high-level behavior.

## Severity

- `blocking`: risky behavior or bug fix has no meaningful validation and could regress silently.
- `important`: tests exist but miss core behavior, edge cases, or error paths.
- `suggestion`: a test name, fixture, or assertion could better describe behavior.

## Output

Report only behavioral test findings. Name the behavior that needs coverage and describe the test that would prove it. If nothing matches, write `No findings for behavioral test coverage.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not demand tests for mechanical docs-only changes.
- Do not require brittle tests that assert private implementation details.
