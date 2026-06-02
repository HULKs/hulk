# Agent-Assisted Review

Use the `agent-review` skill for local pre-review checks before a human reviewer spends time on a branch. The skill runs focused criteria and reports findings only.

Agent findings help reviewers. They do not replace human judgment.

## Skill Discovery

The skill lives at `.agents/skills/agent-review/SKILL.md`, so Codex, Zed, and opencode can discover it directly.

Claude Code discovery is not configured yet.

## How To Run It

By default, `agent-review` runs all review criteria in `.agents/skills/agent-review/references/`.

Useful prompts:

| Goal | Example prompt |
|---|---|
| Full branch review | `Use agent-review to review the current branch against main.` |
| Non-main base | `Use agent-review to review this branch against integration/ros-z.` |
| Commit range | `Use agent-review to review commits abc123..def456.` |
| Staged changes | `Use agent-review to review staged changes only.` |
| Unstaged changes | `Use agent-review to review unstaged changes only.` |
| File subset | `Use agent-review to review only crates/ros-z-streams and docs/framework.` |
| Targeted criteria | `Use agent-review with rust-ownership and api-surface-consistency against main.` |

If the scope is ambiguous, the agent should ask one short clarification. If the base is unavailable, the agent should state the limitation and ask for a base or review only the safe subset.

## What It Checks

| Criterion | Focus |
|---|---|
| duplicate concepts | Reimplemented concepts, missed reuse, and parallel helpers. |
| API surface consistency | Behavior exposed in one surface but missed in CLIs, schemas, messages, docs, config, or runtime paths. |
| Rust ownership | Ownership-sensitive API design, function signatures, clones, borrowing, avoidable allocations, and hot-path data movement. |
| Rust error handling | Panic paths, unwrap, expect, `Result`, error context, and recoverability. |
| Rust type design | Types that should encode invariants with enums, newtypes, and precise states. |
| behavioral test coverage | Missing behavioral tests, weak edge cases, and brittle implementation assertions. |
| docs and examples | Missing README, docs, examples, PR testing notes, and user-facing updates. |
| change minimality | Generated churn, unrelated refactors, over-abstraction, and excessive file movement. |
| architecture fit | Crate placement, dependency direction, node and framework conventions, and migration consistency. |
| config and deployment | `etc/`, `etc/parameters`, TOML and JSON defaults, deployment files, and runtime compatibility. |
| semantic usefulness | Vacuous changes, incomplete behavior, unused features, and non-useful mechanical edits. |
| atomicity and splitting | Changes that should be split into smaller commits or PRs with clearer review boundaries. |

## Output

The skill returns findings in this shape:

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

- Reviewed: actual requested scope.
- Criteria: criteria actually reviewed.
- Mention areas intentionally not inspected.
```

If there are no findings, the skill writes `No findings for the reviewed criteria.`

## Using Results

Before asking for human review:

1. Deduplicate repeated findings.
2. Fix clear `blocking` findings.
3. Decide whether `important` findings should be fixed or explained in the PR.
4. Leave uncertain trade-offs for the human reviewer with the agent evidence attached.

Do not treat agent findings as automatic merge gates. The reviewer decides product intent, domain correctness, and trade-offs.

## Maintenance

Shared review behavior belongs in `.agents/skills/agent-review/SKILL.md`. Criterion-specific checks belong in `.agents/skills/agent-review/references/`.

When changing criteria, run a small pressure scenario before and after the edit. Keep checks that produce concrete, focused findings. Rewrite or remove checks that produce vague, duplicate, noisy, or low-value reports.
