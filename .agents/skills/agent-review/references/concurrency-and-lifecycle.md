# Concurrency And Lifecycle

## Focus

Find async, threading, channel, lock, cancellation, node-cycle, and resource-lifetime problems that can affect runtime reliability or maintainability.

## What To Check

- Blocking I/O, sleeps, heavy computation, or unbounded work in async tasks, cyclers, callbacks, or hot runtime paths.
- Locks held across awaits, callbacks, logging, I/O, or code that can re-enter.
- Unbounded channels, queues, caches, or task spawning without backpressure, cancellation, or shutdown handling.
- Background tasks without join handling, cancellation, or error propagation when failures matter.
- Resources opened without clear cleanup: files, sockets, robot connections, handles, temporary directories, or subscriptions.
- Runtime lifecycle mismatches where initialization, start, stop, drop, or node-cycle behavior diverges from nearby conventions.

## Severity Guidance

- `blocking`: realistic deadlock, task leak, startup or shutdown failure, unbounded memory growth, or robot-runtime stall.
- `important`: lifecycle or concurrency behavior is fragile and likely to create production or maintenance risk.
- `suggestion`: a clearer cancellation, cleanup, or lifecycle pattern would improve readability.

## Output Guidance

Report only concurrency and lifecycle findings. Name the runtime path and explain the concrete lifecycle or concurrency risk. If nothing matches, write `No findings for concurrency and lifecycle.`

## Criterion-Specific Do Not

- Do not require async, threading, or elaborate cancellation abstractions for simple synchronous code.
- Do not flag bounded, short-lived, or local concurrency without evidence of a leak, stall, deadlock, or lifecycle mismatch.
- Do not demand broad runtime rewrites when a small cleanup, bound, join, or cancellation fix solves the issue.
