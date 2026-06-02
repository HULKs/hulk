# Tests Behavioral Coverage

## Focus

Find changed behavior that lacks meaningful tests, and find tests that lock implementation details instead of user-visible or caller-visible behavior.

## What To Check

- New behavior without a test or explicit reason testing is infeasible.
- Bug fixes without a regression test that would have failed before the fix.
- Tests that assert private helper names, file layout, call counts, or migration progress instead of behavior.
- Missing edge cases for empty input, invalid input, boundary values, ordering, timing, and error paths.
- Tests that are nondeterministic, over-mocked, or tightly coupled to incidental implementation.
- PR testing notes that do not explain how the reviewer can validate high-level behavior.

## Severity Guidance

- `blocking`: risky behavior or bug fix has no meaningful validation and could regress silently.
- `important`: tests exist but miss core behavior, edge cases, or error paths.
- `suggestion`: a test name, fixture, or assertion could better describe behavior.

## Output Guidance

Report only behavioral test findings. Name the behavior that needs coverage and describe the test that would prove it. If nothing matches, write `No findings for behavioral test coverage.`

## Criterion-Specific Do Not

- Do not demand tests for mechanical docs-only changes.
- Do not require brittle tests that assert private implementation details.
