# Duplicate Concepts

## Focus

Find added code that reimplements an existing concept instead of reusing, extending, or moving the existing implementation.

## What To Check

- New helpers, structs, traits, algorithms, parameters, or messages that duplicate existing names or behavior.
- Local implementations of concepts that already live in shared crates such as `geometry`, `filtering`, `framework`, `types`, `parameters`, or `ros-z*`.
- Copy-pasted logic with small naming or type changes.
- New abstractions that overlap with existing abstractions but do not replace them.
- Missed extension points where the existing implementation should grow instead of adding a parallel path.

## Severity Guidance

- `blocking`: duplicate behavior will likely diverge, breaks a single source of truth, or conflicts with an existing public abstraction.
- `important`: reuse is straightforward and would reduce maintenance or improve consistency.
- `suggestion`: similar concept exists, but specialization may be justified.

## Output Guidance

Report findings only for duplicate or missed-reuse issues. Include evidence from both the changed code and the existing implementation. If nothing matches, write `No findings for duplicate concepts.`

## Criterion-Specific Do Not

- Do not flag intentional specialization when the diff clearly explains why reuse is wrong.
- Do not require large refactors when a small call into existing code solves the issue.
