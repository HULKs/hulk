# Agent-Assisted Review

Agent review skills are local pre-review checks. Run them before a human reviewer spends time on a branch. Each skill checks one narrow quality criterion and reports findings only.

Agent findings help reviewers. They do not replace human judgment.

## Review Scope

Tell each agent exactly what to review. If no scope is provided, review the current branch against `main`.

Useful scopes:

| Scope | Example prompt |
|---|---|
| Branch diff | `Review the current branch against main with review-rust-ownership.` |
| Non-main base | `Review this branch against integration/ros-z with review-api-surface-consistency.` |
| Commit range | `Review commits abc123..def456 with review-atomicity-and-splitting.` |
| Staged changes | `Review staged changes only with review-change-minimality.` |
| Unstaged changes | `Review unstaged changes only with review-docs-and-examples.` |
| File subset | `Review only crates/ros-z-streams and docs/framework with review-architecture-fit.` |

If the scope is ambiguous, the agent should ask one short clarification. If the base is unavailable, the agent should state the limitation and ask for a base or use the explicit scope.

## Report Format

Each skill reports only findings for its criterion.

```markdown
## Findings

- `blocking|important|suggestion` `path:line`: concise title
  Evidence: what in the diff or repository shows the problem.
  Impact: why it matters.
  Proposed fix: specific remediation.

## Open Questions

- Include only questions needed to resolve uncertainty.

## Scope Notes

- Reviewed: actual requested scope.
- Mention areas intentionally not inspected.
```

If a skill has no findings, it should write `No findings for the reviewed criterion.`

## Severity

| Severity | Meaning |
|---|---|
| `blocking` | Likely bug, broken public contract, unsafe migration, misleading behavior, or reviewability problem that should stop merge. |
| `important` | Quality, maintainability, consistency, or test gap that should usually be fixed before merge. |
| `suggestion` | Small improvement or question that may be worth addressing but should not block by itself. |

## Starter Pack

| Skill | Focus |
|---|---|
| `review-duplicate-concepts` | Reimplemented concepts, missed reuse, and parallel helpers. |
| `review-api-surface-consistency` | Behavior exposed in one surface but missed in CLIs, schemas, messages, docs, config, or runtime paths. |
| `review-rust-ownership` | Clones, borrowing, avoidable allocations, and hot-path data movement. |
| `review-rust-error-handling` | Panic paths, unwrap, expect, `Result`, error context, and recoverability. |
| `review-rust-type-design` | Types that should encode invariants with enums, newtypes, and precise states. |
| `review-tests-behavioral-coverage` | Missing behavioral tests, weak edge cases, and brittle implementation assertions. |
| `review-docs-and-examples` | Missing README, docs, examples, PR testing notes, and user-facing updates. |
| `review-change-minimality` | Generated churn, unrelated refactors, over-abstraction, and excessive file movement. |
| `review-architecture-fit` | Crate placement, dependency direction, node and framework conventions, and migration consistency. |
| `review-config-and-deployment` | `etc/`, `etc/parameters`, TOML and JSON defaults, deployment files, and runtime compatibility. |
| `review-semantic-usefulness` | Vacuous changes, incomplete behavior, unused features, and non-useful mechanical edits. |
| `review-atomicity-and-splitting` | Changes that should be split into smaller commits or PRs with clearer review boundaries. |

## Running A Local Review

Run skills independently. Parallel agents work well because each skill has a narrow focus.

Suggested local pass before human review:

```text
Review the current branch against main with review-duplicate-concepts.
Review the current branch against main with review-api-surface-consistency.
Review the current branch against main with review-rust-ownership.
Review the current branch against main with review-rust-error-handling.
Review the current branch against main with review-tests-behavioral-coverage.
Review the current branch against main with review-change-minimality.
Review the current branch against main with review-atomicity-and-splitting.
```

Add domain-specific skills when the diff touches docs, config, deployment, framework code, node wiring, or public APIs.

## Combining Reports

Before asking for human review:

1. Deduplicate repeated findings.
2. Fix clear `blocking` findings.
3. Decide whether `important` findings should be fixed or explained in the PR.
4. Leave uncertain trade-offs for the human reviewer with the agent evidence attached.

Do not treat agent findings as automatic merge gates. The reviewer decides product intent, domain correctness, and trade-offs.

## Subagent Boundaries

Review agents should not edit files. They should not run formatters or broad test suites unless explicitly asked. They should not perform a general code review. They should not invent findings from style preferences. They should prefer small practical fixes over large rewrites.

## Skill Validation

When adding or changing a skill, run a small pressure scenario before and after the skill text exists.

Baseline prompt:

```text
Do a quick pre-review of this synthetic diff. Keep the review short. Do not use any review-* skill. Return only issues you would raise before a human review.
```

Skill prompt:

```text
Read the skill file being tested and follow it exactly. Review the same synthetic diff. Return findings only for that skill's criterion.
```

Keep skills that produce concrete, focused findings. Rewrite or remove skills that produce vague, duplicate, noisy, or low-value reports.

## Future Backlog

Future skills can cover security, performance, concurrency, observability, dependency hygiene, feature flags, unsafe Rust, schema compatibility, migration cleanup, naming, logging quality, and stale temporary code.
