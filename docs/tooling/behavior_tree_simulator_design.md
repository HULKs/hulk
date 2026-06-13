# Behavior Tree Simulator Design

This document describes a simulator for the current behavior tree instantiated by `crates/nodes/behavior_node/src/tree.rs:create_tree()`.

The simulator initializes the behavior blackboard, repeatedly evaluates the behavior tree, and updates simulated world state and persistent behavior state between ticks.

# Goals

- Run the exact behavior tree returned by `create_tree()`.
- Reuse the production blackboard construction, communication planning, and motion command assembly semantics from `crates/nodes/behavior_node/src/node.rs`.
- Simulate multiple robots from the start.
- Update world state with simple deterministic kinematics after each behavior tick.
- Support Rust scenario programs first.
- Support an interactive Twix/Bevy layer for inspecting and editing simulation state.
- Record behavior traces, motion commands, world states, blackboard-derived debug outputs, and scenario assertions.
- Check simulator invariants every cycle with access to complete simulation state.
- Provide an extensible auto-referee that can update game-controller state from simulated events.

# Non-Goals

- Full physics simulation.
- Accurate NAO motion execution.
- Replacing Webots or hardware integration.
- Simulating perception pipelines in detail.
- Changing behavior tree semantics.
- Implementing new behavior actions as part of the simulator.
- Implementing every HSL rule in the initial auto-referee. Game-state transitions are in scope first; penalties, free-kick correctness, and detailed ball-out rules can follow later.

# Existing Runtime Shape

The production behavior cycle already has the shape the simulator needs:

- The behavior node creates `create_tree()` and the static tree layout.
- The behavior node updates persistent ball memory on its `Blackboard`.
- The behavior node fills a `Blackboard` from parameters, `WorldState`, and previous blackboard state.
- `Node::tick_with_trace()` evaluates the tree and mutates the blackboard.
- `assemble_motion_command()` converts behavior status plus blackboard partial motions into a `MotionCommand`.
- The persistent `Blackboard` keeps selected state between cycles.
- The behavior node publishes trace and debug outputs.

The simulator should not use the retired `world_state::behavior` tree. It should store one `behavior_node::node::Blackboard` and one `behavior_node::behavior_tree::Node` per robot and tick that same tree directly.

# High-Level Architecture

The simulator should be Bevy-based like the old `crates/bevyhavior_simulator`, but it should not restore the old generated cycler/database stack.

The simulator has two layers:

- A behavior adapter over `crates/nodes/behavior_node` that can tick `create_tree()` and plan communication without ROS/network side effects.
- A Bevy runtime in `crates/bevyhavior_simulator` that owns entities, resources, systems, scenario registration, simple kinematics, invariant checks, and timeline recording.

The Bevy runtime is the normal simulator API. A small non-Bevy helper may exist for tests, but scenarios should be authored against Bevy `App` so they can register systems flexibly.

Do not abbreviate `Simulator`, `Simulated`, or `Simulation` to `Sim` in type names. Prefer names such as `SimulatorRobot`, `SimulatorTimeline`, `SimulatorIncomingMessages`, and `SimulatedBall`. Avoid names such as `SimRobot`, `SimTimeline`, or `SimBall`.

# Behavior Tick API

The simulator owns a small adapter around `crates/nodes/behavior_node`.

The adapter input should be small and mirror the data already used by the behavior node loop:

```rust
pub struct SimulatorBehaviorTickInput {
    pub world_state: WorldState,
    pub field_dimensions: FieldDimensions,
    pub behavior_parameters: BehaviorParameters,
}

pub struct SimulatorBehaviorTickOutput {
    pub motion_command: MotionCommand,
    pub trace: NodeTrace,
    pub static_layout: NodeTrace,
    pub path_obstacles: Vec<PathObstacle>,
    pub time_since_last_switch: Duration,
    pub direction_difference: f32,
    pub walk_position: Option<Point2<Ground>>,
    pub voronoi_map: Option<VoronoiGrid>,
    pub voronoi_inputs: Vec<Pose2<Field>>,
}

impl SimulatorRobotBehavior {
    pub fn tick_behavior_tree(&mut self, input: SimulatorBehaviorTickInput) -> Result<SimulatorBehaviorTickOutput>;
}
```

The production behavior node remains the ROS adapter:

- Read framework inputs.
- Fill the production `Blackboard`.
- Tick `behavior_node::tree::create_tree()`.
- Plan outgoing communication through `Blackboard` methods.
- Send planned network messages through the production hardware interface.
- Fill framework outputs.
- Store `last_motion_command` on the `Blackboard`.

The simulator should call its adapter directly and avoid ROS node/cache construction.

# Communication API

Simulations need access to communication, but communication should stay pure and outside the behavior-tree state machine cycle.

Message creation is pure on `behavior_node::node::Blackboard`: `game_controller_return_message()` and `state_message()` return `OutgoingMessage`s and update cooldown fields, but do not write to a network interface.

```rust
impl SimulatorRobotBehavior {
    pub fn plan_communication(
        &mut self,
        world_state: WorldState,
        hsl_network_parameters: HslNetworkParameters,
        game_controller_address: Option<SocketAddr>,
    ) -> Vec<OutgoingMessage>;
}
```

The simulator should call the same `Blackboard` communication methods, route HSL messages to simulated teammates, expose game-controller return messages to scenarios, and record all outgoing messages in the timeline.

This keeps behavior tree evaluation independent from communication side effects while preserving the production cooldown semantics stored in the `Blackboard`.

# Persistent Behavior State

Each simulated robot owns one `SimulatorRobotBehavior` with one `behavior_node::node::Blackboard`. This preserves the same state as production:

- `ball`
- `last_ball`
- `last_close_enough_to_kick`
- `last_kick_target`
- `last_motion_switch_time`
- `last_motion_type`
- `last_sent_game_controller_return_message_time`
- `last_sent_hsl_message_time`

The simulator also stores `last_motion_command` per robot because production keeps it as cycler state.

# Simulation State Model

The Bevy runtime owns simulator state as components and resources. A convenience `Simulation` wrapper may exist, but it should be implemented in terms of the same data model and systems.

Simulator-owned physical state is stored in `coordinate_systems::World`, not `Field`. `World` is the neutral field coordinate system: it is identical to `Field` for the home team and rotated by 180 degrees for the away team. Before building each robot's behavior `WorldState`, the simulator converts the robot pose, ball, and other world-owned physical values into that robot team's `Field` frame using `GameControllerState::global_field_side`.

For HULKs behavior, `GlobalFieldSide::Home` means `World == Field`; `GlobalFieldSide::Away` means `Field` is `World` flipped by 180 degrees. Auto-referee logic that reasons about physical events, such as goals and stationary robots, uses `World`; behavior-tree inputs and outputs remain in `Field`/`Ground` exactly as production expects.

Core resources:

```rust
pub struct SimulatorClock {
    pub now: SystemTime,
    pub tick_duration: Duration,
}

pub struct SimulatorBall {
    pub state: Option<SimulatedBall>,
}

pub struct SimulatorGameState {
    pub game_controller_state: GameControllerState,
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
}

pub struct AutoRefereeConfig {
    pub ready_duration: Duration,
    pub whistle_to_playing_delay: Duration,
    pub halftime_duration: Duration,
    pub auto_whistle_in_set: bool,
    pub finish_on_halftime_timeout: bool,
}

pub struct SimulatorAutoReferee {
    pub rules: Vec<Box<dyn AutoRefereeRule>>,
    pub last_game_state_change: SystemTime,
    pub halftime_started_at: Option<SystemTime>,
    pub playing_after_whistle_at: Option<SystemTime>,
    pub restart_reason: Option<SimulatorRestartReason>,
}

pub struct SimulatorRuleObstacles {
    pub obstacles: Vec<RuleObstacle>,
}

pub struct SimulatorTimeline {
    pub frames: Vec<SimulatorFrame>,
}

pub struct SimulatorScenarioResult {
    pub failed: bool,
    pub failures: Vec<SimulatorFailure>,
}

pub struct SimulatorIncomingMessages {
    pub messages: Vec<SimulatorMessage>,
}

pub struct SimulatorOutgoingMessages {
    pub messages: Vec<SimulatorMessage>,
}

pub struct SimulatorReceivedHslMessages {
    pub messages_by_receiver: BTreeMap<PlayerNumber, BTreeMap<PlayerNumber, SimulatorReceivedHslMessage>>,
    pub player_states_by_receiver: BTreeMap<PlayerNumber, Players<Option<PlayerState>>>,
}

pub struct SimulatorReceivedHslMessage {
    pub message: HulkMessage,
    pub received_at: SystemTime,
}
```

`SimulatorGameState` keeps both the full `GameControllerState` and the filtered state consumed by behavior. Auto-referee systems should mutate the full state through helper methods and then synchronize the filtered state so behavior sees a consistent game-controller view in the same tick.

`AutoRefereeConfig` is intentionally separate from `SimulationConfig`. `SimulationConfig` controls simulator physics, perception, communication, and kinematics. `AutoRefereeConfig` controls HSL rule timing and game-controller transitions.

Robot entities use components and a bundle:

```rust
pub struct SimulatorRobot {
    pub player_number: PlayerNumber,
}

pub struct SimulatorRobotBehavior {
    pub tree: behavior_node::behavior_tree::Node<behavior_node::node::Blackboard>,
    pub blackboard: behavior_node::node::Blackboard,
    pub static_layout: NodeTrace,
}

pub struct SimulatorRobotParameters {
    pub behavior: BehaviorParameters,
}

pub struct SimulatorSuggestedSearchPosition {
    pub position: Option<Point2<Field>>,
}

pub struct SimulatorRobotBundle {
    pub robot: SimulatorRobot,
    pub ground_to_world: SimulatorGroundToWorld,
    pub primary_state: SimulatorPrimaryState,
    pub behavior: SimulatorRobotBehavior,
    pub parameters: SimulatorRobotParameters,
    pub fall_down_state: SimulatorFallDownState,
    pub suggested_search_position: SimulatorSuggestedSearchPosition,
}
```

The non-Bevy convenience wrapper has this shape:

```rust
pub struct Simulation {
    pub now: SystemTime,
    pub tick_duration: Duration,
    pub robots: Players<Option<SimulatedRobot>>,
    pub ball: Option<SimulatedBall>,
    pub game: SimulatedGameState,
    pub field_dimensions: FieldDimensions,
    pub rule_obstacles: Vec<RuleObstacle>,
    pub config: SimulationConfig,
}
```

Each robot has:

```rust
pub struct SimulatedRobot {
    pub player_number: PlayerNumber,
    pub ground_to_world: Isometry2<Ground, World>,
    pub primary_state: PrimaryState,
    pub behavior: SimulatorRobotBehavior,
    pub parameters: BehaviorParameters,
    pub fall_down_state: Option<FallDownState>,
    pub perceived_ball: Option<BallState>,
    pub suggested_search_position: Option<Point2<Field>>,
}
```

The shared ball has world pose and velocity:

```rust
pub struct SimulatedBall {
    pub position: Point2<World>,
    pub velocity: Vector2<World>,
    pub field_side: Side,
}
```

# Per-Tick Loop

Each simulation tick runs these steps in order through Bevy systems:

1. Advance `SimulatorClock::now` by `tick_duration`.
2. Update shared ball position and velocity using simple friction.
3. Run auto-referee systems that consume shared simulation truth such as ball-in-goal and update game-controller state before behavior input is built.
4. Apply routed incoming HSL network messages from the previous tick to per-robot receive state.
5. For each robot, derive robot-local perception inputs from shared simulation state and received teammate messages.
6. For each robot, build a `WorldState` and tick `behavior_node::tree::create_tree()` through `SimulatorRobotBehavior`.
7. Store each robot's `MotionCommand`, `NodeTrace`, and debug outputs.
8. Plan outgoing communication with `behavior_node::node::Blackboard` communication methods using the live message budget in `WorldState.filtered_game_controller_state`.
9. Route planned HSL messages to teammates and decrement the live game-controller message budget.
10. Run invariant checks with access to the full pre-kinematics tick state and all behavior outputs.
11. Apply simple kinematic effects of each `MotionCommand` to robot poses and ball state.
12. Record a frame for scenarios and future viewers, including any invariant failures.
13. Run scenario assertions/hooks.

Tree ticking should be logically simultaneous for all robots. Kinematic updates should use the motion commands from the same tick after all robots have evaluated behavior.

# Bevy Plugin and System Sets

The simulator should provide a Bevy plugin:

```rust
pub struct BehaviorTreeSimulatorPlugin {
    pub config: SimulationConfig,
    pub auto_referee_config: AutoRefereeConfig,
    pub field_dimensions: FieldDimensions,
    pub enable_default_ball_physics: bool,
    pub enable_default_kinematics: bool,
    pub enable_default_communication_routing: bool,
    pub enable_default_invariant_checks: bool,
}
```

The plugin must expose public system sets so scenarios can register systems between any simulator phases:

```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum BehaviorTreeSimulatorSet {
    AdvanceTime,
    BeforeBallPhysics,
    BallPhysics,
    AfterBallPhysics,
    BeforeAutoReferee,
    RunAutoReferee,
    AfterAutoReferee,
    BuildTeamContext,
    BeforeWorldState,
    BuildWorldStates,
    AfterWorldState,
    BeforeBehavior,
    TickBehaviorTrees,
    AfterBehavior,
    BeforeCommunication,
    PlanCommunication,
    AfterCommunication,
    BeforeKinematics,
    ApplyKinematics,
    AfterKinematics,
    BeforeInvariantChecks,
    RunInvariantChecks,
    AfterInvariantChecks,
    RecordTimeline,
    Scenario,
}
```

The plugin should configure these sets in a deterministic chain. Scenarios can insert custom systems with `.in_set(...)`, `.before(...)`, and `.after(...)`.

Examples:

```rust
app.add_systems(
    Update,
    custom_ball_physics.in_set(BehaviorTreeSimulatorSet::BeforeKinematics),
);

app.add_systems(
    Update,
    rewrite_outgoing_messages.in_set(BehaviorTreeSimulatorSet::AfterCommunication),
);

app.add_systems(
    Update,
    scenario_assertions.in_set(BehaviorTreeSimulatorSet::Scenario),
);
```

This is required for scenarios that implement custom physics, inject observations, modify incoming communication, delay outgoing communication, drop messages, duplicate messages, or inspect behavior output before kinematics are applied.

# Robot-To-Robot Communication Routing

The simulator should route HSL robot-to-robot messages through Bevy resources instead of using a network interface. Message creation remains pure and owned by `behavior_node::node::Blackboard` methods.

Default routing semantics:

- Treat `OutgoingMessage::Hsl(HulkMessage)` as a broadcast packet sent by one robot.
- Deliver each HSL packet to every spawned `SimulatorRobot` except the sender.
- Do not deliver self messages. This matches production filtering where `filtered_message` excludes packets from the same player number.
- Ignore `OutgoingMessage::GameController(...)` for robot-to-robot delivery. Scenarios may inspect these messages through `SimulatorOutgoingMessages`.
- Apply routed messages on the next simulator tick. This keeps all robots' behavior ticks logically simultaneous and avoids same-tick feedback loops.
- Store the last received HSL message per `(receiver, sender)` for inspection.
- Convert received state messages into persistent per-receiver `PlayerState`s so teammate state remains available when no new packet arrives on a later tick.

The live HSL message budget is owned by `SimulatorGameState.game_controller_state.hulks_team.remaining_amount_of_messages`.

`SimulationConfig::remaining_amount_of_messages` is only an initial value for that live game-controller field. The plugin should copy it into `SimulatorGameState` during initialization. After startup, the simulator must not use `SimulationConfig::remaining_amount_of_messages` as the source of truth for planning or routing.

Communication planning should expose the current live budget through `WorldState.filtered_game_controller_state.remaining_number_of_messages`:

```rust
let remaining_amount_of_messages =
    game_state.game_controller_state.hulks_team.remaining_amount_of_messages;
```

Routing should handle the budget authoritatively:

- If the live budget is greater than zero, route one HSL broadcast and decrement the budget by exactly one.
- A broadcast to multiple receivers still costs one message, not one message per receiver.
- If the live budget is zero, drop the HSL packet and do not route it.
- Do not decrement the HSL message budget for `OutgoingMessage::GameController(...)`.
- After decrementing, call `SimulatorGameState::sync_filtered_game_controller_state()` so the next `WorldState.filtered_game_controller_state.remaining_number_of_messages` is consistent.

`build_world_states` should construct teammate `WorldState::player_states` from the receiver's persisted communication-derived `PlayerState`s rather than from ground-truth robot poses. A `HulkMessage::State` maps to `PlayerState` on receipt as follows:

- `state_message.pose` becomes `PlayerState::pose`.
- `state_message.ball_position` becomes `PlayerState::ball_position`.
- Ball age should be interpreted relative to the message receive time, matching `PlayerStatesReceiver` semantics.

The robot's own `WorldState::robot.ground_to_field` remains simulator truth. Teammate poses should come from communication so scenarios can test lost, delayed, dropped, duplicated, or stale HSL packets.

# Auto-Referee

The auto-referee should be a Bevy resource plus ordered rules. It should be extensible, but the first expansion should focus on game-state transitions only.

Rule sources:

- `../HSL-Rules/rules/game.tex`
- `../HSL-Rules/common/variables.tex`
- `../HSL-Rules/figs/game_states/game_states.tex`

Relevant state flow from the rules:

- `Initial -> Ready`
- `Ready -> Set`
- `Set -> Playing`
- `Playing -> Ready` for restarts after events such as goals or dropped ball.
- `Playing -> Finished` at half end.

Current protocol types provide `GameState::{Initial, Ready, Set, Playing, Finished}`. Brief stop should be represented through `GameControllerState::stopped`, not as a separate game state. Timeout should be represented through `GamePhase::Timeout`, not as a separate game state.

The auto-referee config should be a standalone Bevy resource:

```rust
pub struct AutoRefereeConfig {
    pub ready_duration: Duration,
    pub whistle_to_playing_delay: Duration,
    pub halftime_duration: Duration,
    pub auto_whistle_in_set: bool,
    pub finish_on_halftime_timeout: bool,
}

impl Default for AutoRefereeConfig {
    fn default() -> Self {
        Self {
            ready_duration: Duration::from_secs(45),
            whistle_to_playing_delay: Duration::from_secs(10),
            halftime_duration: Duration::from_secs(10 * 60),
            auto_whistle_in_set: true,
            finish_on_halftime_timeout: true,
        }
    }
}
```

Use `halftime_duration` naming rather than `period_duration`.

The auto-referee state should be owned separately from the game-controller state:

```rust
pub struct SimulatorAutoReferee {
    pub rules: Vec<Box<dyn AutoRefereeRule>>,
    pub last_game_state_change: SystemTime,
    pub halftime_started_at: Option<SystemTime>,
    pub playing_after_whistle_at: Option<SystemTime>,
    pub restart_reason: Option<SimulatorRestartReason>,
}

pub enum SimulatorRestartReason {
    KickOffAfterGoal { scoring_team: Team },
    DroppedBall,
}
```

Future restart reasons may include initial kick-off, second-half kick-off, penalty kick, and free kick.

The rule trait should receive all state required for game-state transitions:

```rust
pub trait AutoRefereeRule: Send + Sync {
    fn apply(&mut self, context: &mut AutoRefereeContext<'_>);
}

pub struct AutoRefereeContext<'a> {
    pub now: SystemTime,
    pub config: &'a AutoRefereeConfig,
    pub field_dimensions: FieldDimensions,
    pub game_state: &'a mut SimulatorGameState,
    pub auto_referee: &'a mut SimulatorAutoReferee,
    pub ball: &'a mut SimulatorBall,
}
```

`SimulatorGameState` should expose small mutation helpers:

- `set_game_state(game_state, now)`
- `set_kicking_team(kicking_team)`
- `set_stopped(stopped)`
- `sync_filtered_game_controller_state()`

These helpers keep the full `GameControllerState` and `FilteredGameControllerState` synchronized. Scenarios may still mutate resources directly when necessary, but default auto-referee rules should use helpers.

Default auto-referee rules should run in this order:

1. `ScoredGoalRule`
2. `GameStateTransitionRule`
3. `HalftimeTimeoutRule`

`ScoredGoalRule`:

- Run only while `GameState::Playing`.
- If the ball is inside either goal, increment the scoring team's score.
- Remove the ball.
- Set `kicking_team` to the opponent of the scoring team.
- Set `restart_reason = Some(KickOffAfterGoal { scoring_team })`.
- Transition to `GameState::Ready`.
- If the score difference reaches 10, transition to `GameState::Finished` instead.

`GameStateTransitionRule`:

- Transition `Ready -> Set` after `ready_duration`.
- On entering `Set`, place the ball at the center mark with zero velocity for kick-off and dropped-ball restarts when placement is required.
- In `Set`, if `auto_whistle_in_set` is enabled, schedule `playing_after_whistle_at = now + whistle_to_playing_delay`.
- Transition `Set -> Playing` after the scheduled whistle-to-playing time elapses.
- Clear `playing_after_whistle_at` and restart metadata after entering `Playing`.
- Start `halftime_started_at` when entering `Playing` if no half is currently running.

`HalftimeTimeoutRule`:

- If `finish_on_halftime_timeout` is enabled and the game is `Playing`, transition to `Finished` after `halftime_duration` has elapsed since `halftime_started_at`.
- Do not implement the ball-stop extension initially. That can be added later behind a separate config field.

The simulator should default to `GameState::Playing` so simple behavior scenarios and smoke tests start immediately. Scenarios that need full match flow can explicitly set `Initial`, `Ready`, or `Set`.

Scenario control can be added through a Bevy message API:

```rust
pub enum SimulatorRefereeCommand {
    SetGameState(GameState),
    Whistle,
    BriefStop,
    Resume,
    DroppedBall,
    SetTimeout(bool),
}
```

Initial command behavior:

- `SetGameState` applies a direct state override through `SimulatorGameState` helpers.
- `Whistle` schedules `Playing` while in `Set`.
- `DroppedBall` sets restart reason and transitions to `Ready`.
- `BriefStop` sets `GameControllerState::stopped = true`.
- `Resume` clears `stopped`.
- `SetTimeout(true)` sets `GamePhase::Timeout`; `SetTimeout(false)` restores the previous phase or `Normal`.

Detailed free-kick legality, kick-off two-touch restrictions, penalties, ball-out classification, penalty shootout ranking, local/global game-stuck detection, and ball-stop half extension are out of scope for this first game-state-transition expansion.

# WorldState Construction

For each robot, construct `WorldState` with:

- `now` from simulation time.
- `robot.ground_to_field` converted from simulated `ground_to_world` using `GlobalFieldSide`.
- `robot.player_number` from simulated robot identity.
- `robot.primary_state` from robot or simulated game state.
- `ball` from robot perception, not directly from shared truth.
- `rule_ball` from shared truth when rule logic needs it.
- `player_states` from the receiver's persisted HSL-derived teammate state.
- `filtered_game_controller_state` from `SimulatorGameState`.
- `fall_down_state` from the simulated robot.
- `suggested_search_position` from scenario or search model.
- `obstacles` from other robots and scenario obstacles.
- `rule_obstacles` from simulated game/rule state.
- `hypothetical_ball_positions` from scenario or a simple lost-ball model.
- `position_of_interest` from scenario defaults or UI input.

# Blackboard Initialization

Blackboard initialization should stay inside `SimulatorRobotBehavior::tick_behavior_tree()` and mirror production exactly:

- Copy `field_dimensions`, parameters, and `WorldState` into the blackboard.
- Initialize transient debug outputs to empty or zero.
- Keep persistent behavior state on the blackboard: `ball`, `last_ball`, `last_close_enough_to_kick`, `last_kick_target`, `last_motion_switch_time`, `last_motion_type`, communication cooldowns, and `last_motion_command`.
- Reset transient command fields: `is_injected_motion_command`, `walk_position`, `body_motion`, `head_motion`, and `voronoi_map`.

After the tick, leave persistent fields on the blackboard as production does.

# Simple Kinematics

Simple kinematics should be deterministic and configurable. Accuracy is less important than stable, understandable behavior tests.

Use a `SimulationConfig` for constants:

- `walk_translation_speed`
- `walk_rotation_speed`
- `walk_with_velocity_scale`
- `kick_ball_speed_by_power`
- `kick_cooldown`
- `ball_friction_per_second`
- `ball_visibility_range`
- `ball_visibility_angle`
- `robot_radius`

Use invented defaults initially, but keep them compile-time configurable through a plain Rust config struct with a `Default` implementation. Scenario code can construct `SimulationConfig` directly or use `SimulationConfig { field: value, ..Default::default() }`. Do not require parameter files for these constants in the first version.

Initial defaults:

```rust
pub struct SimulationConfig {
    pub walk_translation_speed: f32,
    pub walk_rotation_speed: f32,
    pub walk_with_velocity_scale: f32,
    pub kick_ball_speed_rumpelstilzchen: f32,
    pub kick_ball_speed_schlong: f32,
    pub kick_cooldown: Duration,
    pub ball_friction_per_second: f32,
    pub ball_visibility_range: f32,
    pub ball_visibility_angle: f32,
    pub robot_radius: f32,
    pub kick_radius: f32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            walk_translation_speed: 0.25,
            walk_rotation_speed: 1.0,
            walk_with_velocity_scale: 1.0,
            kick_ball_speed_rumpelstilzchen: 2.0,
            kick_ball_speed_schlong: 4.0,
            kick_cooldown: Duration::from_millis(750),
            ball_friction_per_second: 0.4,
            ball_visibility_range: 4.0,
            ball_visibility_angle: std::f32::consts::FRAC_PI_2,
            robot_radius: 0.25,
            kick_radius: 0.25,
        }
    }
}
```

`MotionCommand::Walk`:

- Move the robot along the first usable segment of the path in ground coordinates.
- Clamp translation by `walk_translation_speed * dt`.
- Rotate toward the command target orientation or path direction.
- Clamp rotation by `walk_rotation_speed * dt`.
- Transform the pose delta into world coordinates and update `ground_to_world`.

`MotionCommand::WalkWithVelocity`:

- Integrate commanded local velocity and angular velocity for `dt`.
- Clamp by simulator speed limits.

`MotionCommand::VisualKick`:

- If the shared ball is within a configured kick radius of the expected ball position, set ball velocity along the kick direction.
- Map `KickPower` to velocity through `SimulationConfig`.
- Enforce `kick_cooldown` per robot to avoid applying a kick every tick while the command remains active.

`MotionCommand::Stand`, `Prepare`, and `StandUp`:

- Do not move the robot.
- `StandUp` clears simulated recovery state after a configured duration or immediately in the first version.

Ball update:

- `position += velocity * dt`.
- Apply exponential or linear friction from `ball_friction_per_second`.
- Optionally clamp or bounce at field borders only if a scenario enables that rule.

# Perception Model

The first perception model should be intentionally simple:

- A robot sees the ball if it is within `ball_visibility_range` and inside `ball_visibility_angle` relative to the robot orientation.
- If visible, set `WorldState::ball` with ground and field positions plus field-side metadata.
- If not visible, set `WorldState::ball` to `None`; persistent `Blackboard::ball` and `Blackboard::last_ball` handle timeout behavior.
- Other robots may become obstacles; teammate `player_states` entries come from received HSL state.
- Scenario code can override visibility, ball observations, hypothetical ball positions, fall state, game state, and search position.

# Multi-Robot Behavior

Multi-robot support is required from the start.

The core should simulate robots together instead of running independent single-robot worlds because behavior depends on team context:

- `player_states` should contain every teammate state received over simulated HSL.
- `is_goalkeeper` depends on `BehaviorParameters::goal_keeper_number`.
- Search/support behavior can use teammate positions and Voronoi inputs.
- Closest-to-ball behavior currently returns `true`; the simulator should still provide correct inputs so a future implementation can be tested without simulator changes.

The simulator routes HSL messages between robots and builds `player_states` from persisted received state rather than ground truth. This keeps communication loss, delay, and staleness testable.

# Rust Scenario API

Rust scenarios should be the first authoring interface and should keep the old Bevy scenario shape:

```rust
#[scenario]
fn intercept_ball(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update.in_set(BehaviorTreeSimulatorSet::Scenario));
}
```

The scenario macro or runner should create an `App`, add `BehaviorTreeSimulatorPlugin`, call the scenario function so it can register arbitrary Bevy systems, then run the app to completion.

The API should support:

- Spawn robots through `Commands` using `SimulatorRobotBundle`.
- Set shared ball position and velocity through `SimulatorBall`.
- Set primary game state and sub-state through resources/components.
- Set goalkeeper number and behavior parameters per robot.
- Register systems in any public simulator system set.
- Wait until a predicate is true by writing normal Bevy systems that send `AppExit`.
- Assert last motion command, trace path, robot pose, ball pose, communication, or role behavior.
- Inject per-tick hooks for dynamic events.
- Disable default physics, kinematics, communication routing, or invariant checks when a scenario provides custom systems.

Example shape:

```rust
#[scenario]
fn striker_walks_to_visible_ball(app: &mut App) {
    app.add_systems(Startup, spawn_robots_and_ball);
    app.add_systems(
        Update,
        assert_striker_walks.in_set(BehaviorTreeSimulatorSet::Scenario),
    );
}
```

Scenarios must be able to modify communication between simulator phases:

```rust
app.add_systems(
    Update,
    inject_teammate_message.in_set(BehaviorTreeSimulatorSet::BeforeWorldState),
);

app.add_systems(
    Update,
    delay_outgoing_messages.in_set(BehaviorTreeSimulatorSet::AfterCommunication),
);
```

The old `#[scenario] fn(app: &mut App)` flexibility is a requirement, not an implementation detail.

# Invariant Checks

All simulator runs must support invariant checks that execute every cycle. These checks validate properties that should hold independent of a specific scenario assertion.

Invariant checks must have access to the complete simulator state:

- Current simulation time.
- Shared ball state.
- All robot poses and persistent robot simulation state.
- Per-robot `WorldState` inputs built for this tick.
- Per-robot behavior outputs, including `MotionCommand`, `NodeTrace`, path obstacles, walk target, Voronoi output, and planned communication.
- Field dimensions, rule obstacles, scenario configuration, and `SimulationConfig`.

The API should be simple Rust code:

```rust
pub trait InvariantCheck {
    fn check(&mut self, snapshot: &SimulationSnapshot) -> Vec<InvariantViolation>;
}

pub struct InvariantViolation {
    pub check_name: &'static str,
    pub player_number: Option<PlayerNumber>,
    pub message: String,
    pub severity: InvariantSeverity,
}
```

Invariant failures must not abort the scenario. They should:

- Mark the current timeline frame with the violation.
- Mark the scenario result as failed.
- Allow the scenario to continue until its normal end condition.
- Be included in the final scenario error/report after the timeline has been finalized.

Initial checks should include:

- A robot should not knowingly try to walk into a rule obstacle.
- A robot should not knowingly try to walk outside the field.

"Knowingly" means the prohibited target or path is visible in the data passed to behavior for that tick, such as `WorldState::rule_obstacles`, field dimensions, planned walking path, or path-obstacle debug output. The check should not fail for hidden state that the robot could not have known from its inputs.

# Viewer and Twix Integration

Viewer work is deferred for now.

The simulator should still record timeline frames in a format that can later be served to Twix or another viewer. The viewer must not reconstruct blackboards or tick behavior directly. It should consume recorded `SimulatorFrame` data.

# Recording and Outputs

Each recorded frame should include:

- Simulation time.
- Shared ball state.
- Robot poses and primary states.
- Per-robot perceived `WorldState` summary.
- Per-robot `MotionCommand`.
- Per-robot planned outgoing communication.
- Per-robot `NodeTrace`.
- Invariant violations for the frame.
- Static behavior tree layout once per run.
- Path obstacles.
- Walk target position.
- Voronoi map and inputs.
- Scenario soft errors and assertions.

The frame format should be serializable so it can be served to Twix and saved for debugging failed scenarios.

Scenario failures must still produce a viewable timeline. The runner should always finalize and serve or save the recording before returning the scenario error. Failed assertions and invariant violations should be attached to recorded frames, and the timeline should include all frames up to the normal scenario end or timeout.

# Integration with Existing Bevyhavior Simulator

Keep the `crates/bevyhavior_simulator` crate name and the old Bevy scenario ergonomics, but replace the old internals:

- `behavior_node` owns behavior-tree semantics, blackboard state, motion command assembly, and communication planning.
- `crates/bevyhavior_simulator` owns Bevy resources, components, systems, scenario assertions, deterministic world updates, communication routing, invariant checks, timeline recording, and scenario binaries.
- Old generated cycler/database code should not be restored.
- Existing scenario binaries can migrate gradually to the new `BehaviorTreeSimulatorPlugin` and `SimulatorRobotBundle` APIs.

# Testing Strategy

Use small scenario tests for behavior branches:

- Safe state selects prepare or stand behavior.
- Stop state stands.
- Initial and Penalized stand and look ahead.
- Fallen robot selects stand-up.
- Set state looks at ball and stands.
- Ready state walks to kickoff pose.
- Playing goalkeeper stands and looks at ball.
- Playing with no perceived ball enters search behavior.
- Playing closest robot walks to ball or kicks when close.
- Supporter computes Voronoi output and walks to centroid when possible.

Use deterministic fixtures:

- Fixed tick duration.
- Fixed parameters.
- Fixed initial poses.
- No random perception.

# Implementation Status

Implemented phases:

1. Tick `behavior_node::tree::create_tree()` through a simulator adapter without using the retired `world_state::behavior` tree.
2. Add `BehaviorTreeSimulatorPlugin`, public `BehaviorTreeSimulatorSet`s, and Bevy resources/components in `crates/bevyhavior_simulator`.
3. Add `SimulatorRobotBundle` and startup helpers for scenarios.
4. Add Bevy systems for multi-robot `WorldState` construction, behavior ticking, communication planning, and trace recording.
5. Add default simple kinematics for walk, kick, stand, prepare, and stand-up, with plugin switches to disable them.
6. Add invariant check support and initial rule-obstacle and field-boundary checks.
7. Add Rust scenario helpers and first branch-coverage scenarios using `#[scenario] fn(app: &mut App)`.
8. Add timeline finalization on normal completion and failure.
9. Add default auto-referee goal scoring and game-state transition rules.
10. Add viewer/server integration for timeline inspection.
