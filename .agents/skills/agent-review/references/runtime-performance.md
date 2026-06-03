# Runtime Performance

## Focus

Find obvious or evidenced runtime performance risks that affect timing, latency, throughput, memory growth, determinism, readability, or maintainability.

## What To Check

- Algorithmic complexity increases in changed hot paths, especially unnecessary nested loops or repeated full scans.
- Data structures that are clearly mismatched to the operation, causing avoidable repeated search, insertion, sorting, or allocation.
- Logging, allocation, serialization, filesystem access, network calls, parameter lookup, or representation conversion repeated inside cycles when one boundary conversion or startup computation would suffice.
- Unbounded memory growth in queues, maps, caches, retained messages, or accumulated diagnostics.
- Work that is recomputed every cycle despite unchanged inputs or an existing cheaper boundary.
- Performance changes that make code harder to understand or review without a clear need.

## Severity Guidance

- `blocking`: realistic timing, memory growth, or runtime stall risk in a robot, simulator, stream, or hot-path execution path.
- `important`: avoidable runtime cost is clear from the changed path and the fix improves maintainability or keeps behavior reviewable.
- `suggestion`: a small data-structure, precomputation, or boundary-conversion change would improve clarity with likely runtime benefit.

## Output Guidance

Report only runtime performance findings. Cite the changed hot path or repeated work and explain why the risk is concrete rather than speculative. If nothing matches, write `No findings for runtime performance.`

## Criterion-Specific Do Not

- Do not ask for full optimization, profiling, benchmarking, or micro-optimization without evidence of a concrete risk.
- Do not fight Rust or the compiler for theoretical gains when the code is clear and the cost is not on a relevant path.
- Do not prefer faster code that is harder to review or maintain unless the performance risk is concrete.
