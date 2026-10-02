# Overview

Robot behavior runs in the ROS-Z `behavior_node`. The node receives the latest
game, robot, localization, ball, obstacle, and team state, ticks the behavior
tree every 20 ms, and publishes the resulting motion command and network
messages.

The behavior tree chooses *what* the robot should do. On main `0711900e0`,
`booster_sdk_interface` consumes `behavior/motion_command` and issues SDK
mode, movement, kick, and get-up requests. A separate head-motion node publishes
head targets for that interface as
described in [motion](../motion/overview.md#ros-z-booster-path).

## Behavior Tree

The tree is built once when `behavior_node` starts. Its main building blocks
are:

- **Conditions**, which inspect the current state and succeed or fail.
- **Actions**, which select body or head motion or update intermediate state.
- **Sequences**, which run children in order and stop at the first failure.
- **Selections**, which try children in order and stop at the first success.
- **Negations**, which invert the result of a child.
- **Subtrees**, which group related decisions under a descriptive name.

The order of children in a selection matters: the first successful branch wins
for that tick. This is how the root tree chooses behavior for the current
primary state and how the playing tree assigns the robot's current task.

## Root Tree and Primary States

The root is one selection whose branches are evaluated from top to bottom. Most
branches first check the robot's `PrimaryState`:

- **`Damping`** selects damping motion.
- **`Prepare`** uses motion-switch timing to request preparation, with a
  centered-head standing alternative while the transition is blocked.
- **`Stop`** stands.
- **Remote control**, when enabled, takes control before normal game behavior.
  It first requests stand-up when SDK recovery is available; otherwise it can
  command walking velocity or a kick with a connected controller, and stands
  when disconnected.
- **Injected motion command**, when configured, provides a direct behavior
  output. It is mainly useful for development and testing.
- **`Finished`** and **`Penalized`** stand.
- **`Initial`** stands and looks around.
- **Stand-up** requests recovery when the SDK fall state's
  `is_recovery_available` is true, before Set, Ready, and Playing.
- **`Set`** stands while looking at the ball or searching for it with the head.
- **`Ready`** runs the ready subtree: walk to the kickoff pose and look around.
- **`Playing`** runs the playing subtree described below.

If no branch succeeds, behavior falls back to standing with a centered head.
Injected commands are returned directly by the motion assembler. Their branch
precedes the normal stand-up branch, while remote control handles recovery in
its own subtree. Main has no shared fall-status/Motion-execution safety branch
or the independent execution gates described in the
[simulator development safety handover](../motion-safety.md).

## Playing

The playing subtree evaluates these branches in order:

1. **Penalty shootout:** run the dedicated penalty-shootout subtree.
2. **Simple mode or last HULK standing:** search when no retained ball position
   is known; otherwise act as striker. This branch precedes goalkeeper selection.
3. **Goalkeeper:** the configured goalkeeper runs the goalkeeper subtree.
4. **Searcher:** a remaining field player without a retained ball position runs
   the search subtree.
5. **Striker:** calculate the team's Voronoi map and select the field player
   closest to the ball. Timing hysteresis prevents rapid switching between
   striker and supporter.
6. **Supporter:** run the supporter subtree as the remaining alternative.

"Last HULK standing" means that no other player state is present in the
blackboard; it does not directly count physically upright teammates. Missing
or expired teammate state can therefore activate this branch. The blackboard
retains ball information for `behavior_node.ball.last_ball_timeout`, so a brief
loss of the current ball estimate does not immediately start searching.

### Search

The `search_suggestor` combines local, team-message, hypothetical, and rule-based
ball information in a heatmap. Regions in an approximate body-facing field of
view decay when no local ball was seen, and a selected region is published as
`suggested_search_position`.

The current search subtree does **not** walk to that suggestion:
`has_suggested_search_position` and `walk_to_search_position` are commented out
in `src/tree.rs`. Its active body action, `leuchtturm`, turns in place, continuing
the previous turn direction or choosing the side where the ball was last seen.
The head looks toward a hypothetical ball when one is available and otherwise
uses `SearchForLostBall`. Motion-switch timing can select a walking alternative
until the search action is allowed.

### Striker

The striker keeps the head focused on the ball and then chooses among the main
ball-handling behaviors:

- During a game substate such as a free kick, goal kick, corner kick, throw-in,
  or penalty kick, follow the appropriate attacking or blocking behavior.
- If the ball is not close enough, walk to a position from which it can be
  kicked toward the opponent goal.
- If a moving ball can be intercepted, prepare the kick for the predicted
  interception point.
- Otherwise execute the normal visual kick behavior.

The detailed distances, alignments, kick power, and motion-switch rules are
parameters and may change independently of this high-level structure.

### Goalkeeper

The goalkeeper tracks the ball and selects the first applicable goalkeeper
behavior. At a high level it handles game substates, clears a nearby ball,
intercepts a dangerous moving ball, temporarily becomes striker when useful,
moves to an active blocking position when the ball threatens the goal, and
otherwise returns to its default position near the own goal.

### Supporter

The supporter tracks the ball and walks to a support position derived from the
team's Voronoi map. This distributes field players while accounting for known
teammates and obstacles. If no support position can be produced, it stands.

## Team Communication

While playing, behavior periodically creates a State message. It contains the
player number, the robot pose, and the observed ball position and age when a
ball is available. A message is sent only when the robot has a field pose, the
send interval has elapsed, and the remaining game message budget is high
enough.

Received teammate states provide the poses used for closest-to-ball selection
and supporter positioning. Team communication can be absent or delayed, so the
tree retains branches for simple operation, missing teammate state, and missing
ball information. See [HSL network](../perception/hsl_network.md) for the UDP
bridge, teammate expiry, GameController traffic, and the status of team-ball
aggregation.

## Motion Output

Behavior actions choose body and head motion independently. After each
successful tree tick, the motion assembler combines both into one
`MotionCommand`. If an action does not select body or head motion, standing and
a centered head are used as defaults. The command is published on
`behavior/motion_command` for the Booster SDK interface.

## Configuration and Inspection

Behavior parameters are loaded from
`etc/parameters/base/behavior_node.json5`, with location- and robot-specific
overrides applied by the ROS-Z runtime. Parameters control details such as
motion switching, kickoff poses, walking, kicking, goalkeeper positioning,
search, Voronoi positioning, and message timing.

The node publishes additional topics for inspection in Twix and recordings:

- `behavior/tree_layout`: the static tree structure
- `behavior/trace`: the result of each tree tick
- `behavior/blackboard`: the inputs and intermediate state used by the tree

Most of the implementation lives in `crates/nodes/behavior_node`:

- `src/tree.rs` defines the root, playing, search, striker, and supporter trees.
- `src/goalkeeper.rs` defines goalkeeper behavior.
- `src/behavior_tree.rs` defines tree evaluation.
- `src/node.rs` connects ROS-Z inputs and outputs and ticks the tree.
- `src/send_message.rs` creates outgoing team messages.
- `crates/nodes/search_suggestor` builds the ball-search heatmap and publishes
  the suggested search position.

Conditions and actions are split into the remaining modules by behavior area.
For exact thresholds and currently active lower-level decisions, the source and
runtime parameters are more authoritative than this overview.
