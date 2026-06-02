# API Surface Consistency

## Focus

Find changes that add behavior in one surface but forget another surface that users, tools, robots, or generated code rely on.

## What To Check

- Public Rust exports, preludes, builders, traits, messages, schemas, and generated code.
- CLI flags, completions, examples, README snippets, and docs pages.
- Parameter defaults, config files, runtime wiring, deployment files, and robot-specific paths.
- Consistent names, defaults, units, validation rules, and error behavior across surfaces.
- Migration paths where old and new surfaces temporarily coexist.

## Severity Guidance

- `blocking`: a feature is unreachable, silently misconfigured, or exposed with conflicting behavior across surfaces.
- `important`: a likely user or tool surface is missing or inconsistent.
- `suggestion`: naming or documentation could align better across surfaces.

## Output Guidance

Report only surface consistency findings. Name the changed surface, the missing or inconsistent surface, and the expected relationship. If nothing matches, write `No findings for API surface consistency.`

## Criterion-Specific Do Not

- Do not require every internal helper to become public.
- Do not assume a surface must exist without evidence that the repository already exposes similar features there.
