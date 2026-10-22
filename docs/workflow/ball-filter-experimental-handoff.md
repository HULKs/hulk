# Experimental ball-filter freshness handoff

Updated: 2026-10-22. **Experimental; not validated for an important game.**

## Intent and current status

In a manual robot test, removing the physical ball reportedly left a selected
candidate for roughly a minute. There is no recording or exact reproduction.
Keep hypotheses for search and reacquisition, but require recent visual support
before publishing one as the current ball.

This checkpoint implements that distinction and adds downstream cache expiry.
It is pushed for review and further testing, not approved for deployment. Use the
last robot-tested match commit and its tested parameters for an important game;
this investigation has not identified an exact match-proven commit.

## Branches and locations

- Production PR: [HULKs/hulk#2965](https://github.com/HULKs/hulk/pull/2965),
  fork branch `dev/ball-filter-preferred-20261004`, experimental changes after
  `4408fd02065a914fc27da3bd147736d0ef0b7a76`.
- Simulator companion: fork branch
  `dev/ball-filter-preferred-simulator-20261004`, experimental changes after
  `92d281c7f5789591ba40521cbace475f8a57fff1`.
- Both follow-up commits have subjects beginning `EXPERIMENTAL:`. PR #2965 stays
  a draft; predecessor PR #2944 is closed.
- The checkpoint commits are unsigned because the configured 1Password signer
  blocked noninteractive commits. The user's Git signing configuration is unchanged.
- Local checkouts: `/home/schluis/hulk-worktree/ball-filter-pr2965` and
  `/home/schluis/hulk-worktree/ball-filter-simulator`. The original checkout at
  `/home/schluis/hulk-worktree/hulk2` is unchanged.

## Behavior and implementation

- `BallFilterParameters.maximum_output_age` is optional. The base configuration
  explicitly enables **200 ms**. An observation exactly 200 ms old remains
  eligible; an older one does not. Zero requires an observation at the output
  timestamp. Omitted or `null` preserves the historical age-unbounded selection
  policy for old recording replay. Future observation timestamps are rejected.
- `best_hypothesis_for_output` filters eligibility before ranking. A stale track
  with high accumulated support cannot block selection of a fresh candidate.
  Stored validity is accumulated support, not a probability of current presence.
- Primary output, auxiliary publication history, selected diagnostics, filtered
  image overlays, and the companion observer use the freshness rule. Stored
  hypotheses remain visible in diagnostics and available as hypothetical search
  positions under the existing retention rules. The base hypothesis timeout
  remains 20 s; this change does not guarantee retaining a hypothesis that long.
- `ball_state_composer.maximum_ball_age` independently bounds cached local and
  team observations at **4.5 s**, matching the existing team-ball age budget.
  A 100 ms logical timer rechecks caches even when upstream messages stop.
  A stale local estimate cannot hide a recent team observation. Composition
  preserves the original observation timestamp.
- The normal filter age gate uses its logical output time. The composer is the
  additional protection against old cached data when upstream publication stops;
  200 ms is not an end-to-end wall-clock guarantee under input silence.

Relevant files are `crates/nodes/ball_filter/src/{filter,hypothesis,tracker,lib}.rs`,
`crates/nodes/ball_state_composer/src/lib.rs`, the ball-filter parameter definition
in `crates/types/src/parameters.rs`, and the two base JSON5 parameter files.
The companion also updates `tools/ball-filter-tuner/src/observation.rs`.

## Validation and limits

Production validation passed:

```sh
cargo +1.98.1 test --locked -p ball_filter -p ball_state_composer -p team_ball_filter -p obstacle_filter --lib
cargo +1.98.1 clippy --locked -p ball_filter -p ball_state_composer -p team_ball_filter -p obstacle_filter --all-targets -- --deny warnings
cargo +1.98.1 fmt --check -p ball_filter -p ball_state_composer -p types
cargo +1.98.1 check --locked -p twix -p hulk_ros_z
```

There are 150 passing tests: 132 ball-filter, 3 composer, 15 obstacle-filter,
and no team-ball-filter unit tests. Regressions cover selecting past stale high
support, auxiliary freshness, cached local/team expiry, recent-team fallback,
future timestamps, and reacquisition using a retained track. The one-minute
regression deliberately uses a 120 s test retention timeout; production still
uses its original retention settings. These are deterministic component tests,
not a reproduction of the unrecorded manual robot scene.

Companion validation also passed:

```sh
cargo +1.98.1 test --locked -p ball-filter-tuner -p ball_filter -p ball_state_composer --lib
cargo +1.98.1 fmt --check -p ball_filter -p ball_state_composer -p types -p ball-filter-tuner
env -u MUJOCO_STATIC_LINK_DIR -u MUJOCO_DYNAMIC_LINK_DIR \
  MUJOCO_NO_PKG_CONFIG=1 MUJOCO_DOWNLOAD_DIR=/home/schluis/.cache/mujoco-rs \
  cargo +1.98.1 check --locked -p simulate
```

All 176 companion tests passed (41 tuner, 132 filter, 3 composer). These commands
used `CARGO_TARGET_DIR=/home/schluis/hulk-worktree/hulk2/target` and
`CARGO_BUILD_JOBS=4` for builds. The simulator was compile-checked; no new GUI demo
or scenario accuracy benchmark ran.

The first tuner run exposed a pre-existing test that pinned the production
field-boundary decay to 0.5 while the checked-in value was already 2.0. The
fixture now sets its own comparison values and also checks that a legacy
age-unbounded baseline stays age-unbounded during parameter import. Runtime
tuning and the scoring objective are unchanged by this fixture repair.

**No new capture, tuning campaign, or real-robot validation was performed.**
The earlier E120/E125 improvements (2.51% close-position RMSE, 0.46% close-velocity
RMSE, and 14.18% duplicate exposure on 54 fresh synthetic recordings) describe
the policy before this freshness change. They do not validate the 200 ms policy.

The main unresolved risk is lost availability through normal occlusions, head
movements, low detection rates, or pipeline delay. The exact cause of the reported
minute-long stale selection remains unproven. The v11 tuner records false-output
duration, but its final loss uses close position/velocity terms, and its aggregate
quality guard does not directly constrain empty-scene false outputs. Do not treat
the objective as having been repaired by this runtime change.

## Run and inspect the simulator

Use the companion branch; the production PR intentionally excludes the simulator.
Choose an unused output directory:

```sh
cd /home/schluis/hulk-worktree/ball-filter-simulator
RUSTUP_TOOLCHAIN=1.98.1 ./simulator demo --output logs/ball-freshness-experimental-20261022
```

This runs nine scenarios without parameter search and opens the 3D viewer.
Open <http://127.0.0.1:8765> for the image observer. Stored hypotheses may remain
visible after the selected estimate disappears. Use `--headless --exit-after`
for a finite run without the desktop viewer. See `tools/simulate/README.md` for
MuJoCo, ONNX Runtime, model, display, and existing-recording prerequisites.
The commands here are instructions for validation, not claims that a new demo
or benchmark has already run. Restart existing processes to load the new code.

## Next work and rollback

1. Record ball removal with a clear camera view, brief occlusions, looking away,
   reacquisition, and recent-team/stale-local handoff. Inspect source timestamps,
   selected output, auxiliary output, and composed `ball_state` together.
2. Verify both normal message flow and upstream silence. Confirm current output
   expires while stored hypotheses remain useful for search.
3. Compare against the previous policy on identical recordings. Measure false
   outputs, missing duration, close coverage, position/velocity errors, and
   reacquisition latency; include per-scenario occlusion regressions.
4. Select the freshness budget from those results before promoting the PR or
   deploying to a game. Add direct false-output optimization/acceptance coverage
   as a separately evaluated change if tuning is resumed.

To remove this experiment from either branch, identify its `EXPERIMENTAL:`
commit with `git log` and revert that commit. Parent `4408fd020` is the previous
production research revision, not a certified game build. Setting
`maximum_output_age` to `null` only disables the new filter age gate; it does not
undo composer expiry or the rest of the experiment. Do not reset a shared branch
or overwrite unrelated work to roll back.
