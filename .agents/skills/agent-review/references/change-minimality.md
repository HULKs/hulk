# Change Minimality

## Focus

Find changes that make review harder without serving the stated behavior: large generated churn, unrelated cleanup, formatting noise, file moves, and abstractions that are not yet needed.

## What To Check

- Behavior changes mixed with unrelated formatting, renames, movement, or cleanup.
- Generated or machine-written churn that hides small human-authored logic.
- New helpers, traits, layers, or configuration that have only one caller and do not clarify the code.
- Large rewrites where a small localized change would satisfy the requirement.
- File moves or module splits that obscure the actual behavior change.
- Dead code, temporary scaffolding, or unused feature paths added by the change.

## Severity Guidance

- `blocking`: the diff is too noisy to review safely or hides behavior in broad mechanical churn.
- `important`: unrelated edits should be split or removed to keep the PR reviewable.
- `suggestion`: a small simplification would improve readability.

## Output Guidance

Report only minimality and reviewability findings. Identify the unrelated or excessive part and the smaller boundary that would preserve intent. If nothing matches, write `No findings for change minimality.`

## Criterion-Specific Do Not

- Do not reject necessary cross-cutting migrations when the coupling is clear.
- Do not demand minimalism that makes the final code less readable or less correct.
