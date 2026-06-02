# Config And Deployment

## Focus

Find changes where configuration, parameters, defaults, deployment files, or runtime compatibility were not updated with the code.

## What To Check

- New parameters without defaults or matching entries in `etc/`, especially `etc/parameters` and its framework- or middleware-specific parameter sets.
- Parameter files under `etc/parameters`, including ros-z parameter layouts such as `etc/parameters/ros_z` when present, left stale after code changes.
- Renamed or removed config fields without migration, compatibility, or clear failure behavior.
- Changed runtime-facing code references TOML, JSON, deployment, service, or script artifacts that are absent, stale, or inconsistent with existing repository wiring patterns.
- Runtime behavior that differs between robot, simulator, tool, or local development paths.
- New crates, binaries, assets, or generated files missing workspace, packaging, or deployment wiring.
- Unsafe defaults that could surprise robot operation or local tooling.

## Severity Guidance

- `blocking`: runtime startup, deployment, robot behavior, or config loading can fail because artifacts are missing or incompatible.
- `important`: defaults, overrides, or docs are inconsistent and likely to confuse users or reviewers.
- `suggestion`: small config naming, grouping, or example cleanup would improve maintainability.

## Output Guidance

Report only config and deployment findings. Name the code path and the missing or inconsistent artifact. If nothing matches, write `No findings for config and deployment.`

## Criterion-Specific Do Not

- Do not require deployment updates for code that is provably test-only or local-only.
- Do not invent compatibility requirements without evidence from existing repository patterns.
