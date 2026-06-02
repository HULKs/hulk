# Docs And Examples

## Focus

Find user-facing or contributor-facing changes that need documentation, examples, or testing notes before human review.

## What To Check

- New or changed commands, flags, parameters, config, APIs, messages, or workflows without docs.
- Stale examples that still show old defaults, names, paths, or behavior.
- Public Rust APIs without useful doc comments when callers need semantics or invariants.
- Docs that describe what changed but not how to use or validate it.
- Missing reviewer test instructions for features that require manual robot, simulator, tool, or deployment validation.

## Severity Guidance

- `blocking`: stale or missing docs can cause dangerous robot operation, wrong deployment, data loss, or unusable public API.
- `important`: user-facing behavior changed and the obvious docs or examples were not updated.
- `suggestion`: small wording, example, or testing-note improvement would reduce reviewer friction.

## Output Guidance

Report only documentation and example findings. Name the stale or missing doc surface and the change it should reflect. If nothing matches, write `No findings for docs and examples.`

## Criterion-Specific Do Not

- Do not demand documentation for private implementation details with no user-facing effect.
- Do not ask for long prose when a short example or command update is enough.
