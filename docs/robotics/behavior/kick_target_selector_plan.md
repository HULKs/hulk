# Kick Target Selector Implementation Plan

Status: goal geometry, ordinary-striker selection, approach, and kick execution
are integrated. Executable selections are `Shoot { opening }` and
`QuickKick { reason }`. `Dribble` and `Clear` remain preparatory intent vocabulary;
until their corridor validation and executors exist, the selector converts those
proposals to the explicit `QuickKick { NoUsablePlan }` fallback. Passing remains
deferred. This is a focused execution increment, not completion of every stage below.

`strategy::select_ball_action()` owns urgent priority. It selects
`QuickKick { NearOwnGoal | NearOpponentGoal | NearbyRobot }` before ordinary
goal-opening evaluation and ordinary planning-ball bounds rejection when the
robot is at most 3 m from either goal mouth, or another robot is at most 1.5 m
from us. The 3 m tuning initially matches `maximum_shot_distance`, but is an
independent constant. Goal proximity is shortest Euclidean distance to the
goal-mouth segment on the goal line, measured from the robot, not the ball.
Robot pressure uses centre-to-centre distance, including known teammates except
our own broadcast. Unaffiliated robot/person/unknown obstacles count conservatively;
ball and goal-post obstacles do not.

The tree consumes that selection before interception and retains ball-reach hysteresis, motion-switch
gate, and outer game-state, set-play, fall-protection, and striker-role gates.
Ordinary selection uses the existing range clipping and shot-side hysteresis.
Approach uses the selected opening's centre. Execution refines only within that
region against the actual visual-kick ball, current obstacles, and shot range;
it does not independently reselect another opening. A refinement failure becomes
`QuickKick { NoUsablePlan }`, and interception cannot overwrite that fallback.
Urgent/fallback kicks aim at the opponent goal centre only with a reachable ball.
Reach checks use the same visual ball as the kick request when available.
Reachable balls are handled before walking path planning,
so an unavailable approach route cannot delay a quick kick. Unreachable balls
with no route cannot fall through to a kick. Set-play alignment remains ahead
of ordinary play. Path-planning failure no longer fabricates a direct walk.
The fallback does not guarantee an obstacle-free ball trajectory.

New requests default to `quick=true` and allow strong kicks. For targets on the
opponent goal mouth or inside its net, strength uses the requested ball's shortest
distance to that mouth: `strong=true` only above 2 m, and false at or below 2 m.
`strong_kick_min_target_distance` configures this cutoff; non-goal targets retain
the ball-to-target metric. `target_speed` remains independently configurable;
motion inference still uses its maximum policy speed for strong kicks, and soft
kicks still ignore strong/quick. Ongoing kicks retain their previous strong,
soft, quick, and target-speed settings even when conditions or tuning change.

The strategy reads the remembered planning ball, world state, field dimensions,
and `parameters.kicking.strategy` from `Blackboard`. The shared tree starts each
tick by moving `ball_action` to `last_ball_action` and clearing the current value.
Only ordinary striker play selects a current action, so leaving that context
clears selection without duplicating lifecycle logic in live and simulator callers.
The previous action supplies hysteresis while approaching; a continuing kick
retains its committed action, field target, flags, and speed. Shot output remains
a region, not an exact target. Future passing can identify a receiver and region
without changing that representation. Unknown robot affiliation is treated
conservatively; semantic opponent/teammate scoring remains deferred.

## Objective

Replace ordinary striker goal-center targeting with geometric opportunities, tactical evaluation, stable action selection, and execution refinement. The selected intent must guide both approach and kicking.

Geometry determines candidate space. Evaluation estimates its value. Strategy chooses an action and region. Execution selects and validates the concrete approach and kick within that region.

The separation is conceptual: implement it initially as modules within `behavior_node`, using the same behavior tree in production and simulation. Separate ROS nodes are unnecessary for the first implementation.

## Scope and delivery order

1. Establish planning contracts and the motion/ball-coordinate boundary.
2. Implement geometric shot opportunities and stable selection for ordinary striker play.
3. Integrate approach, refinement, continuation, and failure handling.
4. Preserve teammate freshness and obstacle identity needed for semantic risk evaluation.
5. Add passes to conservative reception regions around fresh teammate positions.
6. Add explicit clearance and dribble behavior as their execution capabilities become available.
7. Add receiver prediction and coordinated receiving after measuring kick timing and range.

The first useful release is stages 1–3: obstacle-aware shooting with consistent approach and execution. Later stages extend this implementation; they must not require all four action kinds to exist from the beginning.

Initially preserve the explicit behavior of penalty kicks, goalkeeper clearances, interception, remote control, and set plays. The selector applies to ordinary striker play. Expanding it to another context requires that context to supply its allowed actions and constraints.

Out of scope for the first release: a new physical kick library, foot selection, a learned tactical model, a continuous angular risk field, a new network protocol, and full physics simulation.

## Current integration points

Paths in this document are relative to the repository root.

| Area | Current behavior | Planned change |
| --- | --- | --- |
| `crates/nodes/behavior_node/src/kick/mod.rs` | Refines the selected shot region against the execution ball; explicit urgent/fallback requests aim goalward. Active kicks retain the selected target/settings. | Refine complete corridor-validated plans when more executors are available. |
| `crates/nodes/behavior_node/src/walk.rs` | Ordinary shot approach follows the selected region; special contexts keep their goal-center approach. Continuation retains the committed target. | Extend approach to new action-specific executors. |
| `crates/nodes/behavior_node/src/tree.rs` | Selects ordinary intent before approach/kick branching and gives quick requests priority over interception. Clears current intent each tick. | Preserve this authority/selection/execution boundary for later actions. |
| `crates/nodes/behavior_node/src/conditions.rs` | `is_close_to_ball_aligned()` uses goal-center geometry. | Make alignment consume the relevant plan wherever the new selector is used; retain explicit behavior for unmigrated contexts. |
| `crates/nodes/behavior_node/src/node.rs` | Owns live world-state assembly and persistent blackboard state. | Assemble planning input and maintain selected intent, validity, and debug state. |
| `crates/types/src/obstacles.rs` | Robot obstacles have no team identity, observation age, or uncertainty. | Preserve or expose the evidence needed to distinguish confidently known teammates from unmatched robots. |
| `crates/nodes/obstacle_filter/src/lib.rs` | Network teammate observations become generic robot hypotheses. | Preserve teammate association through fusion before using lower teammate risk. |
| `crates/types/src/world_state.rs` | `PlayerState` contains pose and ball information, without receiver velocity. | Preserve observation age for planning; add prediction inputs only in the later prediction stage. |
| `crates/bevyhavior_simulator/src/behavior_runtime.rs` | Initializes the same blackboard and runs the production tree. | Match new initialization, reset, and persistence semantics. |
| `crates/nodes/booster_sdk_interface/src/control.rs` | Converts target, direction, ball position, and two power modes into Booster kick messages. | Retain this execution capability boundary initially. |

Do not place live planning in `world_state_composer` merely because of its name: the inspected implementation is a placeholder, while the behavior node assembles its working world state.

## Architecture and contracts

```text
World snapshot + action constraints + coarse kick capabilities
    → construct opportunities
    → evaluate opportunities
    → select persistent intent
    → refine approach and kick
    → validate and apply motion command
            └─ failure reason → try another eligible opportunity
```

Use a small `ball_play` module within `behavior_node`, with opportunity construction, evaluation, selection, and refinement functions separated by responsibility. Split source files as the implementation grows. Keep generic geometry in `crates/geometry`; put domain types in `crates/types` when shared with visualization or another crate. Do not introduce a generic planner framework.

### Proposed data responsibilities

Names below describe intended contracts, not a finalized Rust API.

| Concept | Required information |
| --- | --- |
| Planning input | Current time, planning ball, localization transform, obstacles, teammates, field dimensions, action constraints, available kick capabilities. |
| Opportunity | Action-specific region, approximate distance/direction, geometric clearance, obstacle provenance, coarse feasibility, and evaluation inputs. |
| Tactical evaluation | Comparable utility plus named score components and rejection reasons. |
| Selected intent | Action, receiver identity or selected region, selection time, prior target, and commitment state. |
| Resolved kick plan | Target in `Field`, approach pose in `Field`, power mode, and the intent/region it satisfies. |
| Refinement result | A complete valid plan or a reason such as blocked corridor, insufficient range, stale receiver, missing ball/localization, or no legal action. |

Use action-specific variants so a pass always identifies a receiver and a shot always identifies a goal region. Add `Clear` and `Dribble` only when their execution paths exist. Do not represent failure as a clearance action or a fabricated target.

Keep scores and intermediate geometric data transient. Persist only information needed to maintain intent and commitment. An interval's vector index is not a stable identity: match previous shot regions by geometric overlap and target containment; identify receivers by player number. Revalidate after region splits or merges.

### Coordinate and ball contract

- Store persistent targets and approach poses in `Field`; convert to current `Ground` coordinates when building motion commands.
- Express angular opportunities around the ball, with one documented reference axis and angle-wrap convention.
- Use the behavior ball estimate for early planning. Before visual-kick execution, revalidate/refine against the fresh `visual_kick_ball_position` actually sent to Booster.
- Missing localization or a missing fresh execution ball prevents publishing a new kick plan. Existing search and motion-transition behavior handles recovery.
- Verify how target position, kick direction, and `kick_target_offset_angle` jointly affect Booster. The current helper rotates the target while leaving direction unchanged. Document the intended calibration mapping and apply clearance checks to the resulting intended trajectory.
- Simulator kicks currently use direction and power, ignoring target position. Simulation alone cannot settle that calibration contract.

### Coarse capabilities before strategy

Geometry produces collision-clear candidates under the current world estimate, not proof that a kick can execute them. Apply a conservative capability check before tactical selection: supported power modes, approximate useful range, angular tolerance, and rough preparation cost.

Keep the exact target and power choice in refinement. Honor `allow_schlong`; an unavailable power must not make an opportunity appear executable. Initial capability values must come from existing calibration or documented conservative measurements. Do not interpret the existing power-selection distance threshold as a measured maximum range.

## Stage 1 — Define contracts and resolve execution semantics

Work:

- Introduce the minimum shot opportunity, selected intent, resolved plan, and failure-result types.
- Define ordinary-striker applicability and explicit precedence of special behavior contexts.
- Establish the coordinate, ball-source, calibration, and coarse-capability contracts above.
- Separate persistent selection state from per-cycle output state, with matching live and simulator initialization.
- Define a single command-application function that accepts a fully validated plan.

Acceptance:

- Existing motion behavior remains unchanged until the selector is integrated.
- Target/direction/calibration semantics are documented sufficiently to implement shot clearance consistently.
- Unknown physical behavior is identified as a measurement dependency; no speculative kick type or kick-completion signal is introduced.

## Stage 2 — Construct and evaluate shot opportunities

Work:

- Construct the opponent goal-mouth angular interval from the planning ball.
- Account for ball radius and post geometry without applying the same margin twice.
- Reuse `Circle::tangents_with_point()` and segment geometry to subtract obstacle shadows. Clip obstruction to the finite ball-to-goal path so obstacles beyond the goal do not remove valid openings.
- Start with the existing physical obstacle radii plus documented uncertainty margins. Treat robot obstacles uniformly until teammate identity is available.
- Handle overlapping sectors, angle wrapping, a ball inside an exclusion circle, a ball near/on the goal line, and empty or degenerate intervals explicitly.
- Keep physical clearance separate from additional risk margins and power-dependent angular error.
- Retain boundary provenance and relevant nearby obstacles for evaluation.
- Reject openings too narrow for all available kick modes or outside their conservative range envelope.
- Rank eligible shot regions using angular clearance, risk, distance, and approximate preparation effort. Use deterministic tie breaking.

Acceptance:

- An unobstructed goal provides a usable interval; a fully blocked goal provides none.
- Every proposed target lies inside a goal region that satisfies the applicable clearance/error margins.
- Obstacles behind the ball or beyond the goal only affect a path if their physical extent actually intersects it.
- Opportunity construction uses analytic intervals rather than a dense angular grid.

## Stage 3 — Integrate stable selection, approach, and execution

Work:

- Update ordinary-striker planning before approach/kick branching in `tree.rs`.
- Refine a target within the selected shot interval, initially maximizing margin to its usable edges. Prefer the previous valid target when differences are insignificant.
- Derive the approach pose from the ball-to-target direction and the existing stand-off distance.
- Make ordinary-striker walking, relevant alignment checks, kicking, and motion-transition continuation consume the same plan. Preserve dedicated approach behavior for unmigrated contexts.
- Select power using ball-to-target distance and the capability model, then revalidate the resulting trajectory and angular tolerance.
- Construct the resolved plan locally; modify `body_motion` only after success. Behavior-tree failure must not leave a partially initialized visual kick available to another branch.
- Try each eligible candidate at most once per planning cycle if refinement rejects it. If refinement fails, expose `QuickKick { NoUsablePlan }` and use the requested goalward fallback only with a reachable ball and permitted motion switch. It is a deliberate release, not a collision-clear shot. Unreachable balls with no walking route stand rather than fabricating a path or kicking.
- Add score hysteresis for switching valid intents and bounded target refinement during approach.
- Define a conservative commitment policy around entry into visual-kick mode. Treat the existing last-motion state as commanded mode, not proof of physical contact or completion. Confirm which updates/cancellations Booster supports before enabling them during commitment.
- Revalidate hard constraints during commitment. Invalidity bypasses ordinary score hysteresis; motion cancellation must use the supported transition path.
- Reset intent on incompatible game context, loss of striker authority, remote/injected control, loss of required localization, or loss/displacement of the ball that invalidates the plan.

Acceptance:

- Approach orientation and emitted kick follow the same selected opening.
- Small obstacle/ball perturbations do not alternate between equivalent openings every cycle.
- A closing corridor invalidates the plan; commitment cannot preserve an unsafe candidate indefinitely.
- Kick continuation never silently replaces the target with the goal center.
- Special contexts retain their explicit authority and existing behavior.

This completes the first release. A no-plan fallback may be tactically limited until clearance or dribbling is added; expose it in diagnostics rather than disguising it as a valid shot.

## Stage 4 — Preserve teammate evidence and evaluate semantic risk

Work:

- Preserve observation timestamps when importing player states into behavior. Reuse upstream expiry and penalized-player filtering; add pass-specific freshness evaluation where stricter limits are needed.
- Preserve confidently associated teammate identity through obstacle fusion. Association should use position uncertainty and observation age, and should survive valid hypothesis updates/merges without assigning identity to unrelated detections.
- Keep unmatched robots as unknown-team obstacles. Do not relabel every unmatched robot as a confirmed opponent.
- Ensure network and visual observations of one teammate do not become two planning obstacles.
- Retain physical collision clearance for every robot. Apply a smaller additional interception penalty only to confidently known teammates; ambiguous identity remains conservative.
- Evaluate interception influence along the finite corridor, including obstacles near it that do not form interval boundaries.

Acceptance:

- Equal-sized openings can receive different tactical risk scores based on reliable teammate evidence.
- Teammate classification never permits a trajectory through the teammate's physical exclusion region.
- Stale or ambiguous association cannot retain reduced risk indefinitely.
- Live and simulated world inputs expose equivalent identity/freshness semantics; simulated ground truth must not silently provide stronger information than production.

## Stage 5 — Add passes to fresh receiver regions

Work:

- Generate one or more conservative reception regions near each eligible teammate; exclude self and unavailable receivers.
- Use teammate position as the strategic reference, with region size limited by freshness and plausible reachability. Do not implement velocity extrapolation yet.
- Treat the receiver as the endpoint participant, while checking the approach corridor against other obstacles. Avoid planning a trajectory through the receiver's body to a point beyond them.
- Check finite pass corridors and in-field reception regions. Evaluate approximate range, corridor risk, receiver access, field progress, and the quality of the resulting possession.
- Put shot and pass evaluation on a common heuristic utility scale. Document component signs and units; do not label uncalibrated scores as probabilities or compare raw opening width directly with raw pass distance.
- Keep selection at receiver/region level. Refine only selected candidates using analytic corridor boundaries and a small bounded set of interior targets when necessary.
- Recheck range, clearance, and receiver reachability after refinement. Return failures to selection; retain receiver identity during ordinary target adjustment.
- Account for the current supporter behavior, which positions through Voronoi logic rather than a receive-pass request. Restrict initial passes to reception opportunities that do not assume coordinated receiver movement.

Acceptance:

- A blocked shot can yield a pass when its reception opportunity is feasible and useful.
- A pass is rejected when the receiver is stale, unreachable, or screened by an obstacle before the reception region.
- An obstacle beyond the reception region does not incorrectly block the pass.
- A small refinement cannot silently turn a pass into an unrelated clearance or target a different receiver.

## Stage 6 — Add clearance and dribble execution

Implement these as separate increments after shooting and passing work.

- Clearance: generate reachable regions that reduce immediate danger; evaluate turnover and own-goal exposure. Explicitly define when deliberate ball-out is permitted by the action context instead of globally requiring every trajectory to remain in the field.
- Dribble: first establish a supported short-touch/control behavior and its range/error envelope. Do not assume the weaker existing power mode provides controlled dribbling. Then evaluate both the ball corridor and the robot's follow-up path.
- Define urgency as tactical context: emergency clearance may outweigh ordinary possession/progress scores, while an unavailable action remains ineligible.
- Give each action a complete execution and failure path before exposing it to strategy.

Acceptance:

- The robot has a deliberate action when neither shot nor pass is desirable and the new capability is available.
- Clearance is selected for a tactical reason, not because refinement failed.
- Dribble plans preserve a reachable follow-up position and respect robot-path constraints separately from ball-path constraints.

## Stage 7 — Add prediction and coordinated receiving

Dependencies: measured kick range/error, preparation time, ball travel time, and sufficient timestamped teammate observations.

- Estimate receiver movement from distinct timestamped observations or introduce communicated movement intent. Retain uncertainty and cap the prediction horizon.
- Predict to expected kick contact plus ball arrival, including observation age. Handle stationary, sparse, or inconsistent observations conservatively.
- Refine reception position and kick power with a bounded iteration; reject candidates when their timing cannot be made consistent.
- Add pass-intent communication and receiver behavior only when coordinated movement is needed. Include receiver identity, reception region/target, and expiry/cancellation semantics; plan the network-message compatibility change explicitly at that stage.
- Re-evaluate after material receiver or opponent movement, using the same intent/commitment policy.

Acceptance:

- Prediction improves measured reception outcomes over the static-region baseline.
- Packet delay/loss or noisy velocity estimates cause conservative fallback rather than uncontrolled extrapolation.
- Receiving does not depend on an uncommunicated assumption that the teammate will change its current behavior.

## Parameters and diagnostics

Add only parameters consumed by an implemented stage, under the existing behavior parameter structure in `crates/types/src/parameters.rs` and `etc/parameters/base/behavior_node.json5`.

Initial groups: physical/uncertainty margins, kick range and angular tolerance per supported power, selection hysteresis, and permitted target refinement. Later groups: teammate freshness/association and tactical utility weights. Validate finite values, nonnegative margins, and ordered range bounds. Separate measured motion properties from tactical preferences.

Expose through the existing `behavior/blackboard` and simulator diagnostics:

- Candidate goal/reception regions and obstacle associations.
- Feasibility rejection reasons and named utility components.
- Selected intent, previous/current target, approach pose, power, and commitment state.
- Refinement failures, fallback reason, and planning duration/candidate count.

Render these as map overlays when integrating the selector. Preserve the production behavior trace rather than creating a second decision engine in tooling.

## Validation and completion criteria

Use focused geometry tests and deterministic behavior scenarios; avoid broad tests that merely duplicate helper implementations.

| Area | Required evidence |
| --- | --- |
| Geometry | Open/blocked goal, merged sectors, wraparound, tangent/degenerate cases, finite corridor endpoints, post/ball margins. |
| Frames and calibration | Translated/rotated robot poses preserve field targets; final command mapping matches the documented direction/target contract. |
| Capabilities | Disabled powers and insufficient range reject candidates; power changes trigger the appropriate clearance/error checks. |
| Lifecycle | No oscillation under small perturbations; invalidation on corridor closure, ball loss, role/context changes; continuation preserves intent. |
| Failure handling | Every refinement candidate is bounded; all-failed selection emits a deliberate fallback without a partial visual kick. |
| Passing | Self exclusion, stale receivers, receiver at corridor endpoint, teammate deduplication, blocked paths, and unreachable reception regions. |
| Integration | Production and simulator use the same planner/tree and equivalent blackboard reset/persistence semantics. |
| Physical behavior | Hardware measurements for angular error, power/range, preparation time, and target updates/cancellation under visual kick. |
| Runtime | Measure planning within the existing 20 ms behavior cycle under representative obstacle counts; use analytic construction and bounded refinement before considering scheduling changes. |

Run repository-required checks appropriate to each implementation increment, plus targeted tests for affected crates. Broaden checks when shared world-state or parameter types change. Simulator success establishes decision and integration behavior; it does not establish physical pass accuracy.

An increment is complete when its acceptance criteria pass, diagnostics explain selection and rejection, applicable special contexts retain their behavior, and the supported execution capabilities match what strategy can select.

## Reference

The strategic-request/skill-execution separation is inspired by [B-Human's architecture overview](https://docs.b-human.de/master/b-human-at-a-glance/). This plan adapts that separation to the current HULKs behavior tree and Booster motion interface rather than assuming equivalent physical skills.
