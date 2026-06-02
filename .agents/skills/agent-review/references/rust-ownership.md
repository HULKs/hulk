# Rust Ownership

## Focus

Find Rust code and newly introduced API surfaces that fight ownership, allocate unnecessarily, or move data in ways that make code slower or harder to read.

## What To Check

- Unnecessary `.clone()`, `.to_owned()`, `.to_string()`, or intermediate `.collect()` calls.
- New or changed function signatures, public APIs, traits, builders, and message boundaries where ownership choices should be designed deliberately.
- Parameters that should borrow as `&T`, `&str`, or `&[T]` instead of taking owned values, or APIs that should accept owned values because they store, spawn, or transfer ownership.
- Return types that unnecessarily allocate or clone instead of returning references, iterators, `Cow`, or existing borrowed views when those fit the API contract.
- Cloning or allocation inside loops, cyclers, message paths, stream processing, or runtime hot paths.
- Data structures that force ownership where borrowing, iteration, or existing domain types would be clearer.
- Ownership workarounds that hide a simpler API boundary.
- Ownership design issues that Clippy will not catch, especially whether a new API should expose borrowing, ownership transfer, iteration, or cloning at its boundary.

## Severity Guidance

- `blocking`: avoidable hot-path allocation or cloning creates clear runtime risk.
- `important`: ownership choices are likely to hurt maintainability, readability, or performance.
- `suggestion`: a smaller borrow or simpler data flow would improve readability.

## Output Guidance

Report only Rust ownership findings. Include why the current ownership behavior matters at the cited call site or API boundary. If nothing matches, write `No findings for Rust ownership.`

## Criterion-Specific Do Not

- Do not flag clones that are required for ownership, concurrency, lifetime boundaries, or API contracts.
- Do not demand lifetime gymnastics that make the code less readable without a concrete benefit.
- Do not optimize allocations without evidence from the changed path, nearby usage, or the ownership contract introduced by a new API signature.
