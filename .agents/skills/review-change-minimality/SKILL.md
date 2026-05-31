---
name: review-change-minimality
description: Use when reviewing PRs or local diffs for avoidable churn, broad rewrites, unrelated refactors, generated noise, file movement, or over-abstraction.
---

# Review Change Minimality

## Focus

Find changes that make review harder without serving the stated behavior: large generated churn, unrelated cleanup, formatting noise, file moves, and abstractions that are not yet needed.

## Scope

Use the caller-provided review scope. If no scope is provided, compare the current branch with `main`.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate this skill's focus.

## What To Check

- Behavior changes mixed with unrelated formatting, renames, movement, or cleanup.
- Generated or machine-written churn that hides small human-authored logic.
- New helpers, traits, layers, or configuration that have only one caller and do not clarify the code.
- Large rewrites where a small localized change would satisfy the requirement.
- File moves or module splits that obscure the actual behavior change.
- Dead code, temporary scaffolding, or unused feature paths added by the change.

## Severity

- `blocking`: the diff is too noisy to review safely or hides behavior in broad mechanical churn.
- `important`: unrelated edits should be split or removed to keep the PR reviewable.
- `suggestion`: a small simplification would improve readability.

## Output

Report only minimality and reviewability findings. Identify the unrelated or excessive part and the smaller boundary that would preserve intent. If nothing matches, write `No findings for change minimality.`

For each finding, include severity, file and line when possible, evidence, impact, and a proposed fix. Include open questions and scope notes when they affect the review.

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters or broad test suites unless the caller asks.
- Do not report issues outside this skill's focus unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not reject necessary cross-cutting migrations when the coupling is clear.
- Do not demand minimalism that makes the final code less readable or less correct.
