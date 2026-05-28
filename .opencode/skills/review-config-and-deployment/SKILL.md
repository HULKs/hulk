---
name: review-config-and-deployment
description: Use when reviewing PRs or local diffs for parameter drift, missing defaults, etc or etc/parameters changes, config issues, deployment artifacts, or runtime compatibility.
---

# Review Config And Deployment

## Focus

Find changes where configuration, parameters, defaults, deployment files, or runtime compatibility were not updated with the code.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- New parameters without defaults or matching entries in `etc/`, especially `etc/parameters` and its framework- or middleware-specific parameter sets.
- Parameter files under `etc/parameters`, including ros-z parameter layouts such as `etc/parameters/ros_z` when present, left stale after code changes.
- Renamed or removed config fields without migration, compatibility, or clear failure behavior.
- Changed runtime-facing code references TOML, JSON, deployment, service, or script artifacts that are absent, stale, or inconsistent with existing repository wiring patterns.
- Runtime behavior that differs between robot, simulator, tool, or local development paths.
- New crates, binaries, assets, or generated files missing workspace, packaging, or deployment wiring.
- Unsafe defaults that could surprise robot operation or local tooling.

## Severity

- `blocking`: runtime startup, deployment, robot behavior, or config loading can fail because artifacts are missing or incompatible.
- `important`: defaults, overrides, or docs are inconsistent and likely to confuse users or reviewers.
- `suggestion`: small config naming, grouping, or example cleanup would improve maintainability.

## Output

Report only config and deployment findings. Name the code path and the missing or inconsistent artifact. If nothing matches, write `No findings for config and deployment.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not require deployment updates for code that is provably test-only or local-only.
- Do not invent compatibility requirements without evidence from existing repository patterns.
