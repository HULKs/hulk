# Atomicity And Splitting

## Focus

Find changes that would be easier, safer, and faster to review as smaller commits or PRs with clear boundaries.

## What To Check

- Multiple unrelated features in one branch.
- Refactors mixed with behavior changes and formatting churn.
- Generated files mixed with hand-written logic without a separate boundary.
- Tests or docs covering only one of several behavior changes.
- Commit history that makes review harder because each commit is not independently understandable.
- A large migration that could be split by crate, node, API surface, or compatibility layer.

## Severity Guidance

- `blocking`: the change is too broad to review safely or hides risky behavior behind unrelated churn.
- `important`: splitting would materially improve review quality and reduce rework.
- `suggestion`: commit grouping or PR description could clarify the review path.

## Output Guidance

Report only atomicity and splitting findings. Suggest concrete split boundaries such as crates, features, generated output, refactor-first, behavior-second, docs-third, or migration slices. If nothing matches, write `No findings for atomicity and splitting.`

## Criterion-Specific Do Not

- Do not demand splitting for a cohesive cross-cutting change when the coupling is clear.
- Do not block on size alone; explain the independent concerns or review risks.
