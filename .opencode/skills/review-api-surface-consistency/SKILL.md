---
name: review-api-surface-consistency
description: Use when reviewing PRs or local diffs for features exposed inconsistently across APIs, CLIs, schemas, messages, config, docs, or runtime paths.
---

# Review API Surface Consistency

## Focus

Find changes that add behavior in one surface but forget another surface that users, tools, robots, or generated code rely on.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- Public Rust exports, preludes, builders, traits, messages, schemas, and generated code.
- CLI flags, completions, examples, README snippets, and docs pages.
- Parameter defaults, config files, runtime wiring, deployment files, and robot-specific paths.
- Consistent names, defaults, units, validation rules, and error behavior across surfaces.
- Migration paths where old and new surfaces temporarily coexist.

## Severity

- `blocking`: a feature is unreachable, silently misconfigured, or exposed with conflicting behavior across surfaces.
- `important`: a likely user or tool surface is missing or inconsistent.
- `suggestion`: naming or documentation could align better across surfaces.

## Output

Report only surface consistency findings. Name the changed surface, the missing or inconsistent surface, and the expected relationship. If nothing matches, write `No findings for API surface consistency.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not require every internal helper to become public.
- Do not assume a surface must exist without evidence that the repository already exposes similar features there.
