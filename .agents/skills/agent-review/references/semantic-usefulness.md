# Semantic Usefulness

## Focus

Find changes that appear implemented but do not produce useful, reachable, or complete behavior for callers, tools, robots, or maintainers.

## What To Check

- New helpers, types, parameters, or features that no production path uses.
- Tests that verify isolated code while the feature remains unreachable.
- Mechanical rewrites that do not change behavior, readability, safety, or maintainability.
- Partially implemented features with no clear guard, error, documentation, or follow-up boundary.
- Code that computes values but drops them, logs them only, or never exposes them to the intended consumer.
- Claimed behavior in docs or PR text that the diff does not actually implement.

## Severity Guidance

- `blocking`: the PR claims functionality that is unreachable, incomplete, or misleading.
- `important`: meaningful behavior is only partially wired or not validated end to end.
- `suggestion`: the change could better demonstrate its value with a call site, example, or clearer boundary.

## Output Guidance

Report only semantic usefulness findings. State the claimed or implied behavior and the missing path that makes it useful. If nothing matches, write `No findings for semantic usefulness.`

## Criterion-Specific Do Not

- Do not judge product priority without evidence.
- Do not reject preparatory work when the diff clearly documents the boundary and the work is reviewable on its own.
