# Architecture Fit

## Focus

Find changes that do not fit the repository's crate boundaries, node model, framework conventions, dependency direction, or ongoing migration patterns.

## What To Check

- Shared domain types placed inside a node or tool crate instead of `types`, `framework`, `geometry`, or another shared crate.
- Low-level crates depending on tools, binaries, nodes, or higher-level runtime code.
- New crates missing workspace membership, workspace dependency use, or coherent ownership.
- Node changes that ignore established creation, cycle, parameter, input, output, or runtime conventions.
- Migration changes that use names, topics, parameters, or wiring inconsistent with nearby migrated code.
- Boundary leaks where deployment, config, docs, or tooling concerns enter core logic without a clear reason.

## Severity Guidance

- `blocking`: dependency direction, crate placement, or runtime wiring creates a serious architectural or build risk.
- `important`: the change should move to a better crate, layer, or convention before it spreads.
- `suggestion`: naming or placement could align better with adjacent code.

## Output Guidance

Report only architecture-fit findings. Cite the changed location and the existing convention it should follow. If nothing matches, write `No findings for architecture fit.`

## Criterion-Specific Do Not

- Do not propose broad architecture rewrites unrelated to the changed scope.
- Do not block deliberate migrations when the diff documents the transitional boundary.
