# Overview

Robot behavior runs in the ROS-Z `behavior_node`. The node receives the latest
game, robot, localization, ball, obstacle, and team state, ticks the behavior
tree every 20 ms, and publishes the resulting motion command and network
messages.

The behavior tree chooses *what* the robot should do. Motion execution is
handled by the Booster interface, which consumes `behavior/motion_command` as
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
- **`Prepare`** requests preparation motion. While a motion switch is not yet
  allowed, the robot stands and looks forward.
- **`Stop`** stands.
- **Remote control**, when enabled, takes control before normal game behavior.
  It can command walking velocity or a kick.
- **Injected motion command**, when configured, provides a direct behavior
  output. It is mainly useful for development and testing.
- **`Finished`** and **`Penalized`** stand.
- **`Initial`** stands and looks around.
- **Fall recovery** requests stand-up when recovery is available. It is checked
  before the active game-state branches.
- **`Set`** stands while looking at the ball or searching for it with the head.
- **`Ready`** runs the ready subtree: walk to the kickoff pose and look around.
- **`Playing`** runs the playing subtree described below.

If no branch succeeds, behavior falls back to a safe standing command.

## Playing

The playing subtree assigns one of four high-level tasks:

- **Goalkeeper:** the configured goalkeeper runs the goalkeeper subtree.
- **Searcher:** a robot without a known ball position runs the search subtree.
- **Striker:** the last active robot always becomes striker. Otherwise, the
  team's Voronoi map determines which field player is closest to the ball and
  should become striker. Timing hysteresis prevents rapid switching between
  striker and supporter.
- **Supporter:** every remaining field player runs the supporter subtree.

Simple mode bypasses this team-based task assignment: the robot searches when
the ball is unknown and otherwise acts as striker.

### Search

When the ball position is unknown, the `search_suggestor` combines local, team,
hypothetical, and rule-based ball information in a heatmap. Areas the robot has
already observed become less likely, and the strongest remaining region is
published as `suggested_search_position`. The searcher walks there while
looking toward plausible ball positions, then scans the area for the ball. If
no suggested position is available, the robot turns in place and looks around
for the ball, continuing the previous search direction or turning toward the
side where the ball was last seen.

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
tree retains branches for simple operation, the last active robot, and missing
ball information.

## Motion Output

Behavior actions choose body and head motion independently. After each
successful tree tick, the motion assembler combines both into one
`MotionCommand`. If an action does not select body or head motion, standing and
looking forward are used as defaults. The command is published on
`behavior/motion_command` for the Booster interface.

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
