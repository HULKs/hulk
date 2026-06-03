---
name: agent-review
description: Use when reviewing PRs, branch diffs, staged changes, commit ranges, or local diffs for agent-review or retired review-* requests, including rust-ownership clones borrowing collect, rust-error-handling unwrap expect panic, rust-type-design, concurrency lifecycle, runtime-performance, docs examples, config deployment, architecture fit, api-surface-consistency CLI schema consistency, duplicate concepts, semantic usefulness, change minimality, and atomicity splitting.
---

# Agent Review

Run focused pre-review checks before human review. By default, review all criteria listed below. If the caller names criteria, review only those criteria.

## Scope

Use the caller-provided review scope. If no scope is provided, review `git diff $(git merge-base HEAD main)..HEAD`. If local `main` is unavailable, try `origin/main`; if neither base is available, state the limitation and review only the safe subset.

Accepted scopes include branch diffs, non-main bases, commit ranges, staged changes, unstaged changes, and file subsets.

If the scope is ambiguous, ask one short clarification. If the base is unavailable, state the limitation and review only the safe subset.

Within that scope, inspect changed files and enough surrounding repository context to evaluate each selected criterion.

## Criteria

Default criteria:

| Criterion | Reference |
|---|---|
| duplicate concepts | `references/duplicate-concepts.md` |
| API surface consistency | `references/api-surface-consistency.md` |
| Rust ownership | `references/rust-ownership.md` |
| Rust error handling | `references/rust-error-handling.md` |
| Rust type design | `references/rust-type-design.md` |
| concurrency and lifecycle | `references/concurrency-and-lifecycle.md` |
| runtime performance | `references/runtime-performance.md` |
| docs and examples | `references/docs-and-examples.md` |
| change minimality | `references/change-minimality.md` |
| architecture fit | `references/architecture-fit.md` |
| config and deployment | `references/config-and-deployment.md` |
| semantic usefulness | `references/semantic-usefulness.md` |
| atomicity and splitting | `references/atomicity-and-splitting.md` |

Accept common aliases such as `rust-ownership`, `api-surface`, `concurrency`, `lifecycle`, `performance`, `docs`, `config`, `architecture`, `minimality`, `atomicity`, and `duplicates`.

## Execution

When the caller does not name criteria, dispatch one independent subagent for every criterion in the table. Give each subagent:

- The exact review scope.
- The relevant reference file.
- This shared scope, severity, output, and Do Not contract.

For a targeted review, dispatch only the requested criteria. Keep each subagent focused on one criterion.

Each subagent returns only findings for its criterion. Combine reports by deduplicating repeated findings while preserving the strongest severity and clearest evidence.

## Severity

- `blocking`: likely bug, broken public contract, unsafe migration, misleading behavior, or reviewability problem that should stop merge.
- `important`: quality, maintainability, or consistency gap that should usually be fixed before merge.
- `suggestion`: small improvement or question that may be worth addressing but should not block by itself.

## Output

Report only review results using this format. Do not include summaries beyond these sections:

```markdown
## Findings

- `blocking|important|suggestion` `path:line`: concise title
  Criterion: criterion name.
  Evidence: what in the diff or repository shows the problem.
  Impact: why it matters.
  Proposed fix: specific remediation.

## Open Questions

- Include only questions needed to resolve uncertainty.

## Scope Notes

- Reviewed: actual requested scope, including base and head commit SHAs when available.
- Criteria: criteria actually reviewed.
- Mention areas intentionally not inspected.
```

If there are no findings for all reviewed criteria, write `No findings for the reviewed criteria.`

## Do Not

- Do not edit files.
- Do not perform a general code review.
- Do not run formatters, mutate files, or run expensive or broad checks unless the caller asks.
- Cheap, targeted, read-only checks are allowed when they directly support a finding; report what was run and do not treat failures outside the review scope as findings.
- Do not report issues outside the selected criterion unless they are severe and directly evidenced.
- Do not invent findings from style preferences.
- Do not demand large rewrites when a small fix solves the issue.
- Do not treat agent findings as automatic merge gates.
- Do not collapse all criteria into one broad review when independent subagents are available.
