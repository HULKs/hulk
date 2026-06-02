# Rust Type Design

## Focus

Find Rust changes where the type system should carry invariants that are currently enforced by comments, strings, booleans, sentinels, or scattered runtime checks.

## What To Check

- Finite states represented as `String`, integers, or unrelated booleans instead of enums.
- Invalid combinations that could be impossible with a more precise type.
- IDs, units, coordinates, durations, and thresholds that need newtypes or existing domain types.
- Sentinel values where `Option`, `Result`, `NonZero*`, ranges, or enums would be clearer.
- Public structs that expose fields too loosely for downstream callers to use safely.
- Serialization changes that need explicit names, defaults, or `deny_unknown_fields` for configuration-like data.

## Severity Guidance

- `blocking`: the weak type model permits invalid robot, message, or API states with realistic failure impact.
- `important`: a precise enum, newtype, or option would prevent common misuse or remove repeated runtime validation of the same invariant.
- `suggestion`: type clarity could improve readability but current risk is low.

## Output Guidance

Report only type-design findings. Explain the invariant and name a compact Rust type shape that would encode it. If nothing matches, write `No findings for Rust type design.`

## Criterion-Specific Do Not

- Do not propose typestate or complex generics for simple local logic.
- Do not demand newtypes where existing domain types already communicate the invariant clearly.
