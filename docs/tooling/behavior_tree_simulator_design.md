# Behavior Tree Simulator Design

This document designs a simulator for the current behavior tree instantiated by `crates/world_state/src/behavior/tree.rs:create_tree()`.

The simulator must initialize the behavior blackboard, repeatedly evaluate the behavior tree, and update simulated world state and persistent behavior state between ticks.

No implementation is included in this document.

# Goals

- Run the exact behavior tree returned by `create_tree()`.
- Reuse the production blackboard construction and motion command assembly semantics from `crates/world_state/src/behavior/node.rs`.
- Simulate multiple robots from the start.
- Update world state with simple deterministic kinematics after each behavior tick.
- Support Rust scenario programs first.
- Support an interactive Twix/Bevy layer for inspecting and editing simulation state.
- Record behavior traces, motion commands, world states, blackboard-derived debug outputs, and scenario assertions.
- Check simulator invariants every cycle with access to complete simulation state.

# Non-Goals

- Full physics simulation.
- Accurate NAO motion execution.
- Replacing Webots or hardware integration.
- Simulating perception pipelines in detail.
- Changing behavior tree semantics.
- Implementing new behavior actions as part of the simulator.

# Existing Runtime Shape

The production behavior cycle already has the shape the simulator needs:

- `Behavior::new()` creates `create_tree()` and the static tree layout.
- `Behavior::cycle()` updates persistent ball memory.
- `Behavior::cycle()` builds a `Blackboard` from parameters, `WorldState`, previous motion command, and behavior state.
- `Node::tick_with_trace()` evaluates the tree and mutates the blackboard.
- `assemble_motion_command()` converts behavior status plus blackboard partial motions into a `MotionCommand`.
- `Behavior::cycle()` persists selected blackboard fields back into `Behavior`.
- `Behavior::cycle()` publishes trace and debug outputs.

The simulator should not duplicate this logic. It should extract the pure behavior tick from `Behavior::cycle()` and call it from both production and simulation code.

# High-Level Architecture

The simulator should be Bevy-based like the old `crates/bevyhavior_simulator`, but it should not restore the old generated cycler/database stack.

The simulator has two layers:

- A pure behavior adapter in `crates/world_state` that can tick `create_tree()` and plan communication without framework or network side effects.
- A Bevy runtime in `crates/bevyhavior_simulator` that owns entities, resources, systems, scenario registration, simple kinematics, invariant checks, and timeline recording.

The Bevy runtime is the normal simulator API. A small non-Bevy helper may exist for tests, but scenarios should be authored against Bevy `App` so they can register systems flexibly.

Do not abbreviate `Simulator`, `Simulated`, or `Simulation` to `Sim` in type names. Prefer names such as `SimulatorRobot`, `SimulatorTimeline`, `SimulatorIncomingMessages`, and `SimulatedBall`. Avoid names such as `SimRobot`, `SimTimeline`, or `SimBall`.

# Behavior Tick API

Add a pure behavior tick API in `crates/world_state/src/behavior/`.

The API should be small and mirror the data already used by `Behavior::cycle()`:

```rust
pub struct BehaviorTickInput {
    pub world_state: WorldState,
    pub field_dimensions: FieldDimensions,
    pub behavior_parameters: BehaviorParameters,
    pub free_kick_obstacle_radius: f32,
    pub last_motion_command: MotionCommand,
}

pub struct BehaviorTickOutput {
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

impl Behavior {
    pub fn tick_behavior_tree(&mut self, input: BehaviorTickInput) -> Result<BehaviorTickOutput>;
}
```

`Behavior::cycle()` should become a framework adapter:

- Read framework inputs.
- Call `tick_behavior_tree()`.
- Plan outgoing communication with a pure communication API.
- Send planned network messages through the production hardware interface.
- Fill framework outputs.
- Store `last_motion_command`.

The simulator should call `tick_behavior_tree()` directly and avoid framework `CycleContext` construction.

# Communication API

Simulations need access to communication, but communication should stay pure and outside the behavior-tree state machine cycle.

Extract message creation from `send_game_controller_return_message()` and `send_state_message()` into a pure planning API. The API returns outgoing messages and updated communication cooldown state, but does not write to `NetworkInterface`.

```rust
pub struct CommunicationInput {
    pub world_state: WorldState,
    pub game_controller_address: Option<SocketAddr>,
    pub hsl_network_parameters: HslNetworkParameters,
    pub remaining_amount_of_messages: Option<u16>,
}

pub struct CommunicationOutput {
    pub outgoing_messages: Vec<OutgoingMessage>,
    pub last_sent_message: Option<HulkMessage>,
}

impl Behavior {
    pub fn plan_communication(&mut self, input: CommunicationInput) -> CommunicationOutput;
}
```

Production `Behavior::cycle()` should call `plan_communication()` after `tick_behavior_tree()` and write each returned `OutgoingMessage` to hardware. The simulator should call the same method, route HSL messages to simulated teammates, expose game-controller return messages to scenarios, and record all outgoing messages in the timeline.

This keeps behavior tree evaluation independent from communication side effects while preserving the production cooldown semantics stored in `Behavior`.

# Persistent Behavior State

Each simulated robot owns one `Behavior` instance. This preserves the same state as production:

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
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
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
```

Robot entities should use components or a bundle:

```rust
pub struct SimulatorRobot {
    pub player_number: PlayerNumber,
}

pub struct SimulatorRobotBehavior {
    pub behavior: Behavior,
}

pub struct SimulatorRobotParameters {
    pub behavior: BehaviorParameters,
}

pub struct SimulatorLastMotionCommand {
    pub motion_command: MotionCommand,
}

pub struct SimulatorSuggestedSearchPosition {
    pub position: Option<Point2<Field>>,
}

pub struct SimulatorRobotBundle {
    pub robot: SimulatorRobot,
    pub ground_to_field: GroundToField,
    pub primary_state: SimulatorPrimaryState,
    pub behavior: SimulatorRobotBehavior,
    pub parameters: SimulatorRobotParameters,
    pub last_motion_command: SimulatorLastMotionCommand,
    pub fall_down_state: SimulatorFallDownState,
    pub suggested_search_position: SimulatorSuggestedSearchPosition,
}
```

If a non-Bevy wrapper exists, it may have this shape:

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
    pub ground_to_field: Isometry2<Ground, Field>,
    pub primary_state: PrimaryState,
    pub behavior: Behavior,
    pub parameters: BehaviorParameters,
    pub last_motion_command: MotionCommand,
    pub fall_down_state: Option<FallDownState>,
    pub perceived_ball: Option<BallState>,
    pub suggested_search_position: Option<Point2<Field>>,
}
```

The shared ball has field pose and velocity:

```rust
pub struct SimulatedBall {
    pub position: Point2<Field>,
    pub velocity: Vector2<Field>,
    pub field_side: Side,
}
```

# Per-Tick Loop

Each simulation tick runs these steps in order through Bevy systems:

1. Advance `SimulatorClock::now` by `tick_duration`.
2. Update shared ball position and velocity using simple friction.
3. Build `player_states` from all simulated robot poses.
4. For each robot, derive robot-local perception inputs from shared simulation state.
5. For each robot, build a `WorldState` and call `Behavior::tick_behavior_tree()`.
6. Store each robot's `MotionCommand`, `NodeTrace`, and debug outputs.
7. Run invariant checks with access to the full pre-kinematics tick state and all behavior outputs.
8. Apply simple kinematic effects of each `MotionCommand` to robot poses and ball state.
9. Record a frame for scenarios and future viewers, including any invariant failures.
10. Run scenario assertions/hooks.

Tree ticking should be logically simultaneous for all robots. Kinematic updates should use the motion commands from the same tick after all robots have evaluated behavior.

# Bevy Plugin and System Sets

The simulator should provide a Bevy plugin:

```rust
pub struct BehaviorTreeSimulatorPlugin {
    pub config: SimulationConfig,
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

# WorldState Construction

For each robot, construct `WorldState` with:

- `now` from simulation time.
- `robot.ground_to_field` from simulated robot pose.
- `robot.player_number` from simulated robot identity.
- `robot.primary_state` from robot or simulated game state.
- `ball` from robot perception, not directly from shared truth.
- `rule_ball` from shared truth when rule logic needs it.
- `player_states` from all simulated robots.
- `filtered_game_controller_state` from `SimulatedGameState`.
- `fall_down_state` from the simulated robot.
- `suggested_search_position` from scenario or search model.
- `obstacles` from other robots and scenario obstacles.
- `rule_obstacles` from simulated game/rule state.
- `hypothetical_ball_positions` from scenario or a simple lost-ball model.
- `position_of_interest` from scenario defaults or UI input.

# Blackboard Initialization

Blackboard initialization should stay inside `Behavior::tick_behavior_tree()` and mirror production exactly:

- Copy `field_dimensions`, `free_kick_obstacle_radius`, parameters, and `WorldState` into the blackboard.
- Initialize transient debug outputs to empty or zero.
- Copy persistent behavior state into `ball`, `last_ball`, `last_close_enough_to_kick`, `last_kick_target`, `last_motion_switch_time`, and `last_motion_type`.
- Copy simulator-owned `last_motion_command` into the blackboard.
- Reset transient command fields: `is_injected_motion_command`, `walk_position`, `body_motion`, `head_motion`, and `voronoi_map`.

After the tick, persist the same fields back to `Behavior` as production does.

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
- Transform the pose delta into field coordinates and update `ground_to_field`.

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
- If not visible, set `WorldState::ball` to `None`; persistent `Behavior::ball` and `Behavior::last_ball` handle timeout behavior.
- Other robots become obstacles and `player_states` entries.
- Scenario code can override visibility, ball observations, hypothetical ball positions, fall state, game state, and search position.

# Multi-Robot Behavior

Multi-robot support is required from the start.

The core should simulate robots together instead of running independent single-robot worlds because behavior depends on team context:

- `player_states` should contain every simulated teammate.
- `is_goalkeeper` depends on `BehaviorParameters::goal_keeper_number`.
- Search/support behavior can use teammate positions and Voronoi inputs.
- Closest-to-ball behavior currently returns `true`; the simulator should still provide correct inputs so a future implementation can be tested without simulator changes.

If later behavior uses network messages instead of direct `player_states`, add an optional message simulation pass. The first version should prefer direct `WorldState` construction because the behavior tree consumes `WorldState`.

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

- `world_state::behavior` owns pure behavior ticking and pure communication planning.
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

# Implementation Phases

Implementation should wait for an explicit command.

Recommended phases:

1. Extract pure `Behavior::tick_behavior_tree()` from `Behavior::cycle()` without changing behavior semantics.
2. Add `BehaviorTreeSimulatorPlugin`, public `BehaviorTreeSimulatorSet`s, and Bevy resources/components in `crates/bevyhavior_simulator`.
3. Add `SimulatorRobotBundle` and startup helpers for scenarios.
4. Add Bevy systems for multi-robot `WorldState` construction, behavior ticking, communication planning, and trace recording.
5. Add default simple kinematics for walk, kick, stand, prepare, and stand-up, with plugin switches to disable them.
6. Add invariant check support and initial rule-obstacle and field-boundary checks.
7. Add Rust scenario helpers and first branch-coverage scenarios using `#[scenario] fn(app: &mut App)`.
8. Add timeline finalization on normal completion and failure.
9. Add viewer/server integration later.

# Open Questions Before Implementation

No known open questions remain. Implementation should still wait for an explicit command.
