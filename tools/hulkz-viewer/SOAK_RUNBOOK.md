# hulkz-viewer UI Soak Runbook

This runbook validates long-running viewer responsiveness and basic resource stability during continuous ingest.

## 1. Goal

Confirm that:

1. The UI remains interactive (timeline scrub, panel switching, text rendering).
2. Memory usage is stable (no unbounded growth trend after warmup).
3. Ingest and scrub behavior stay correct over extended runtime.

## 2. Prerequisites

1. Build passes:
   - `cargo check -p hulkz-viewer`
   - `cargo test -p hulkz-viewer`
2. Publisher available:
   - `cargo run -p hulkz --example publisher`

## 3. Long-Run Session

Terminal A:

```bash
cargo run -p hulkz --example publisher
```

Terminal B:

```bash
RUST_LOG=hulkz_viewer=info,hulkz_stream=info cargo run -p hulkz-viewer -- --namespace demo --source odometry
```

Recommended duration: 30-60 minutes.

## 4. Interaction Checklist

Repeat every 5-10 minutes:

1. Toggle `Follow live` on/off.
2. Use `Prev` / `Next` / `Jump Latest`.
3. Open and close additional `Text` panels and `Parameters` panel.
4. Rebind a text panel path via completion dropdown.
5. Change default namespace to empty and back to `demo`.
6. Toggle `Ingest` off for 10s, then on again.

Expected:

1. No hangs or panics.
2. Timeline remains navigable.
3. Text panels show valid anchor-aligned values.
4. Discovery refreshes correctly after namespace changes.

## 5. Memory/CPU Observation

Use your preferred system monitor (`htop`, `top`, GNOME System Monitor).

Acceptance guideline:

1. CPU may spike briefly during interaction, then settles.
2. Memory rises during warmup/history accumulation, then stabilizes without sustained runaway growth.
3. No steady linear memory increase over the full soak interval.

## 6. Logging Spot Checks

Look for:

1. Repeated error floods in viewer logs.
2. Frequent `lagged` warnings under normal local load.
3. Repeated bind/unbind failures.

Any persistent issue should be captured with:

1. Exact command line
2. Timestamp window
3. Relevant log excerpt
4. Reproduction steps

## 7. Optional Automated Worker Soak

This checks worker-side continuity (not full UI behavior):

```bash
cargo test -p hulkz-viewer soak_worker_stays_healthy_under_continuous_stream -- --ignored
```
