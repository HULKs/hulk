# Overview

Robot behavior runs in the ROS-Z `behavior_node`.
The node receives the latest game, robot, localization, ball, obstacle, and team state, ticks the behavior tree every 20 ms, and publishes the resulting motion command and network messages.

The behavior tree chooses _what_ the robot should do.
Motion execution is handled by the Booster interface, which consumes `behavior/motion_command` as described in [motion](../motion/overview.md#ros-z-booster-path).

## Behavior Tree

The tree is built once when `behavior_node` starts.
Its main building blocks are:

- **Conditions**, which inspect the current state and succeed or fail.
- **Actions**, which select body or head motion or update intermediate state.
- **Sequences**, which run children in order and stop at the first failure.
- **Selections**, which try children in order and stop at the first success.
- **Negations**, which invert the result of a child.
- **Subtrees**, which group related decisions under a descriptive name.

The order of children in a selection matters: the first successful branch wins for that tick.
This is how the root tree chooses behavior for the current primary state and how the playing tree assigns the robot's current task.

## Root Tree and Primary States

The root is one selection whose branches are evaluated from top to bottom.
Most branches first check the robot's `PrimaryState`:

- **`Damping`** selects damping motion.
- **`Prepare`** requests preparation motion.
  While a motion switch is not yet allowed, the robot stands and looks forward.
- **`Stop`** stands.
- **Remote control**, when enabled, takes control before normal game behavior.
  It can command walking velocity or a kick.
- **Injected motion command**, when configured, provides a direct behavior output.
  It is mainly useful for development and testing.
- **`Finished`** and **`Penalized`** stand.
- **`Initial`** stands and looks around.
- **Fall recovery** requests stand-up when recovery is available.
  It is checked before the active game-state branches.
- **`Set`** stands while looking at the ball or searching for it with the head.
- **`Ready`** runs the ready subtree: walk to the kickoff pose and look around.
- **`Playing`** runs the playing subtree described below.

If no branch succeeds, behavior falls back to a safe standing command.

## Ready Formations

Ready uses static posistions from `behavior_node.kickoff`, expressed in normalized field coordinates and they orientate towards the center of the field.

- The player selected by `goalkeeper.player_number` uses `goalkeeper_position`.
- On our kickoff, the highest-numbered active field player uses `striker_position`.
  The remaining field players fill `aggressive_positions` in array order, from highest to lowest player number.
- On opponent or unknown kickoff, all field players fill `defensive_positions` in the same order.

## Playing

Penalty shootouts use a dedicated subtree.
Otherwise, the playing subtree assigns one of four high-level tasks:

- **Goalkeeper:** the configured goalkeeper runs the goalkeeper subtree.
- **Searcher:** a robot without a known ball position runs the search subtree.
- **Striker:** the team's Voronoi map determines which field player is closest to the known ball and should become striker.
  Timing hysteresis prevents rapid switching between striker and supporter.
- **Supporter:** every remaining field player runs the supporter subtree.

Simple mode and the last active robot bypass this team-based task assignment: the robot searches when the ball is unknown and otherwise acts as striker.

The ball state records whether the position comes from local perception, a teammate, or a rule-generated position, with local perception taking precedence over a teammate.
When the current ball state disappears, behavior briefly retains the last position.
The retention time is controlled by `ball.last_ball_timeout` for local balls and `ball.team_ball_timeout` for team balls.
Rule-generated positions are provided separately and are not retained as last seen balls.

### Search

The `search_suggestor` combines local, team, hypothetical, and rule-based ball information in a priority heatmap with configurable, approximately square cells and values between zero and one.
`cells_along_field_length` fixes the longitudinal cell count; the transverse count is rounded from the field aspect ratio.
Cell dimensions are then calculated independently to cover the field exactly.
Changing resolution at runtime samples the old map at the new cell centres and remaps the selected target.
An observed ball raises nearby cells without clearing the rest of the heatmap; ball positions outside the field are ignored.
After losing a locally observed ball during `Playing`, priorities grow over time, fastest around its last position.
The half-distance of additional last-ball growth is `last_ball_initial_half_distance + last_ball_spread_speed * observation_age`, in metres.
The additional growth adds one unit per `last_ball_extra_priority_rise_time` at the last position before the age weighting is applied.
The spatial bias fades linearly over `last_ball_priority_fade_time` (10 seconds by default), measured from the ball's last observation rather than reception of a predicted filter output.
Setting this duration to zero disables the spatial bias immediately.
Afterwards only uniform growth remains, adding one unit per `unseen_priority_rise_time`; previously accumulated priorities are still cleared by observed empty areas.
On entering `Playing`, a map with a high average priority is scaled down while preserving differences between cells.
Areas within the estimated field of view lose priority; obstacles reduce this decay for areas they obscure.
Observation half-life is interpolated linearly from `observation_half_life_near` to `observation_half_life_far` across `observation_distance_range`.
A visible cell is multiplied by `exp(-ln(2) * elapsed * weight / half_life)`; outside the range there is no observation decay.
`teammate_observation_weight` scales reconstructed team exposure, with zero disabling it, and `occluded_observation_weight` scales locally occluded exposure.
Team replay aims for samples at most `teammate_replay_step` apart, capped at four samples per message interval.
Longer intervals use coarser sampling; the full actual message interval is distributed across the samples.
Rule hypotheses gain one priority unit per `rule_priority_rise_time`, using elapsed simulation time.
Corner kicks, goal kicks, and throw-ins clear the heatmap outside their restart locations, including priorities from earlier observations.

The robot's own pose and fresh teammate poses define its search Voronoi region.
The partition is cached and recomputed only when the grid geometry, owner, or participating players' Voronoi cells change; teammate freshness is checked every cycle.
Field players exclude the configured goalkeeper from this partition, matching the behavior's goalkeeper-free Voronoi calculation.
The goalkeeper searches independently of the field-player partition.
Within that region, the suggestor selects a cell above the minimum priority.
For the configured goalkeeper, search targets are additionally restricted to the own half of the field, including fallback targets.
The suggestor receives the goalkeeper player number from the behavior node; until that assignment is available, it does not publish a search target.
It slightly lowers the selection score of cells within `border_view_penalty_width_fraction * field.width` of the field boundary, where more of the search view would fall outside the field, and prefers smaller turns among similarly ranked cells.
Hysteresis stabilizes the selection.
After a prior local ball sighting, while playing without a current ball, the fallback timer starts when no eligible cell meets `minimum_search_priority`.
After `exhausted_search_delay` without a regular target, the suggestor selects the best remaining cell above `fallback_minimum_search_priority` until a regular target becomes available again.
The selected cell's center is published as `suggested_search_position`, or `None` when no valid target is available.
A robot that is already inside the cell instead keeps its current position and scans without walking to the center.
Setting `search_suggestor.disable_heatmap_search` to `true` stops heatmap updates and publishes no search target, so the existing search behavior turns in place (`leuchtturm`).
Re-enabling heatmap search starts with a fresh map instead of replaying observations collected while it was disabled.

The searcher uses the existing walking behavior to approach that position with `walking.speed.search`, while the head looks toward plausible ball positions or scans for the ball.
The suggestor publishes the physical cell dimensions on the retained `search_cell_size` topic, including after resolution changes.
Behavior uses this geometry without binding a second parameter set.
The arrival distance is `search.position_reached_cell_fraction` times the shorter heatmap cell edge, bounded below by `search.minimum_position_reached_distance` in metres. Within that distance, the robot turns in place and continues searching.
It also turns in place when there is no target or no localization, continuing the previous search direction or turning toward the side where the ball was last seen.
A changed target is used on the next behavior cycle; the heatmap remains responsible for choosing the next search area.
A known ball returns the robot to its normal playing behavior.

### Striker

The striker keeps the head focused on the ball and then chooses among the main ball-handling behaviors:

- During a game substate such as a free kick, goal kick, corner kick, throw-in, or penalty kick, follow the appropriate attacking or blocking behavior.
- If the ball is not close enough, walk to a position from which it can be kicked toward the opponent goal.
- If a moving ball can be intercepted, prepare the kick for the predicted interception point.
- Otherwise execute the normal visual kick behavior.

The detailed distances, alignments, kick power, and motion-switch rules are parameters and may change independently of this high-level structure.

### Goalkeeper

The goalkeeper tracks the ball and selects the first applicable goalkeeper behavior.
At a high level it handles game substates, clears a nearby ball, intercepts a dangerous moving ball, temporarily becomes striker when useful, moves to an active blocking position when the ball threatens the goal, and otherwise returns to its default position near the own goal.

### Supporter

The supporter tracks the ball and walks to a support position derived from the team's Voronoi map.
This distributes field players while accounting for known teammates and obstacles.
If no support position can be produced, it stands.

## Team Communication

While playing, behavior periodically creates a State message.
It contains the player number, the robot pose, the commanded head yaw, and the ball position and age when a ball is available.
A message is sent only when the robot has a field pose, the send interval has elapsed, and the remaining game message budget is high enough.

Each unpenalized player, including the sender, gets an equal share of the remaining message budget after subtracting `remaining_amount_of_messages_to_stop_sending` as a reserve:

```text
time_left = remaining_time_in_half + (half_duration if first_half else 0)
interval_seconds = time_left_seconds * unpenalized_players / usable_messages
```

Behavior publishes the currently calculated interval on `hsl_state_message_send_interval` for the search suggestor to assess teammate freshness and replay observations.
When the game-controller state is unavailable or sending is suspended, the published interval is `None` and teammates are not considered for search regions or replay.

Received teammate states provide the poses used for closest-to-ball selection, supporter positioning, and search regions.
The `player_states` stream keeps the short configured expiry for behavior and other consumers.
The search suggestor instead consumes `last_received_player_states`, which retains timestamped states until they are replaced or the player is penalized. It applies the current HSL send interval as its own freshness limit before using teammates for search regions, ball evidence, or replay.
It interpolates poses and head directions between sufficiently recent states to estimate which areas teammates have observed and lower their search priority.
States with non-finite pose, head yaw, or ball position are discarded before they reach these consumers.
Team communication can be absent or delayed, so the tree retains branches for simple operation, the last active robot, and missing ball information.

## Motion Output

Behavior actions choose body and head motion independently.
After each successful tree tick, the motion assembler combines both into one `MotionCommand`.
If an action does not select body or head motion, standing and looking forward are used as defaults.
The command is published on `behavior/motion_command` for the Booster interface.

## Configuration and Inspection

Behavior parameters are loaded from `etc/parameters/base/behavior_node.json5`, with location- and robot-specific overrides applied by the ROS-Z runtime.
Parameters control details such as motion switching, kickoff poses, walking, kicking, goalkeeper positioning, search, Voronoi positioning, and message timing.

Heatmap priorities, decay, and target selection are configured separately in `etc/parameters/base/search_suggestor.json5`.
`minimum_search_priority` must be finite and between zero and one; `fallback_minimum_search_priority` must be finite and between zero and `minimum_search_priority`.

The node publishes additional topics for inspection in Twix and recordings:

- `behavior/tree_layout`: the static tree structure
- `behavior/trace`: the result of each tree tick
- `behavior/blackboard`: the inputs and intermediate state used by the tree

The search suggestor also exposes `ball_search_heatmap` and `suggested_search_position` for inspecting priorities and the current target.

Most of the implementation lives in `crates/nodes/behavior_node`:

- `src/tree.rs` defines the root, playing, search, striker, and supporter trees.
- `src/goalkeeper.rs` defines goalkeeper behavior.
- `src/search.rs` approaches search targets and turns in place to scan.
- `src/behavior_tree.rs` defines tree evaluation.
- `src/node.rs` connects ROS-Z inputs and outputs and ticks the tree.
- `src/send_message.rs` creates outgoing team messages.

Conditions and actions are split into the remaining modules by behavior area.
`crates/nodes/search_suggestor` connects observations to the search algorithm and publishes its results.
`crates/search_heatmap` implements priority updates, field-of-view decay, rule hypotheses, and target selection.

For exact thresholds and currently active lower-level decisions, the source and runtime parameters are more authoritative than this overview.
