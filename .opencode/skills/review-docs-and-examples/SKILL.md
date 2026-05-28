---
name: review-docs-and-examples
description: Use when reviewing PRs or local diffs for missing documentation, stale examples, outdated README content, or absent reviewer testing notes.
---

# Review Docs And Examples

## Focus

Find user-facing or contributor-facing changes that need documentation, examples, or testing notes before human review.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- New or changed commands, flags, parameters, config, APIs, messages, or workflows without docs.
- Stale examples that still show old defaults, names, paths, or behavior.
- Public Rust APIs without useful doc comments when callers need semantics or invariants.
- Docs that describe what changed but not how to use or validate it.
- Missing reviewer test instructions for features that require manual robot, simulator, tool, or deployment validation.

## Severity

- `blocking`: stale or missing docs can cause dangerous robot operation, wrong deployment, data loss, or unusable public API.
- `important`: user-facing behavior changed and the obvious docs or examples were not updated.
- `suggestion`: small wording, example, or testing-note improvement would reduce reviewer friction.

## Output

Report only documentation and example findings. Name the stale or missing doc surface and the change it should reflect. If nothing matches, write `No findings for docs and examples.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not demand documentation for private implementation details with no user-facing effect.
- Do not ask for long prose when a short example or command update is enough.
