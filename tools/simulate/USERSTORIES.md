# User Stories

## MVP: Full-Stack Simulation

### Running the Simulator

- As a user, I want to run the simulator on a powerful host computer and connect to it remotely.
  - The simulator must support a headless host without a monitor or desktop session.
  - The listen addresses and ports must be configurable so multiple simulator instances can run on one host.
  - A remote disconnect must not silently stop or alter the simulation.
- As a user, I want one command to start a simulation with the selected virtual robots and wait for separately managed robotics stacks to connect.
  - Each virtual robot must have a unique identity and ROS-Z namespace.
  - The namespace must be visible and stable so users can configure a robotics stack, Twix, and `rosz` to connect directly to that virtual robot.
  - Each virtual robot must be controlled by a separate robotics-stack process.
  - Robotics-stack processes are not child processes of the simulator and may run on different host computers.
  - A supported workflow is running the simulator on a powerful server while a robotics stack runs on a developer laptop.
  - The simulator must indicate when the expected nodes and interfaces for a stack are available before transferring control of its robot.
  - The first compatible stack connected through the robot's namespace must receive exclusive actuator ownership.
  - A second stack attempting to control an owned robot must be rejected and produce a visible warning.
  - A disconnected or unavailable stack must be reported without leaving the simulation hanging.
  - Stopping the simulator must release its own network resources cleanly but must not terminate externally managed stack processes.
- As a user, I want simulated robotics stacks to be isolated from physical robot hardware and competition networks.

### Full-Stack Integration

- As a user, I want to run the same robotics code in simulation that runs on a physical robot.
  - Hardware-facing nodes must be replaced by simulator adapters that publish the same ROS-Z data consumed by downstream production nodes and consume the normal production actuator commands.
  - Processing, perception, localization, behavior, and motion components must run unchanged and must not receive simulator ground truth in the MVP.
  - The simulator-adapter boundary must be explicit and shared by all virtual robots.
- As a user, I want the simulator to provide the sensor data required by the full robotics stack.
  - The required MVP hardware data is stereo camera input, IMU and 22 serial-joint states at 500 Hz, and a constant upright fall-down state.
  - Serial motor position, velocity, acceleration, and applied torque must come from MuJoCo.
  - Motor temperature, loss counters, and reserved metadata may use documented constants in the MVP.
  - IMU orientation, angular velocity, and proper linear acceleration must be derived from the simulated rigid body without noise, bias, quantization, or latency in the MVP.
  - The constant upright fall-down state is a known limitation: physically fallen robots will not detect the fall or initiate production stand-up behavior in the MVP.
  - Simulated data must use the same schemas, units, coordinate frames, timestamps, and naming conventions as physical robot data.
  - Sensor rates, capture times, and delivery delays must be defined relative to simulation time.
- As a user, I want actuator commands from the robotics stack to control the corresponding simulated robot.
  - The simulator must consume the existing `booster::LowCommand` with commands for the 22 serial joints.
  - The simulator must expose only this low-level joint-command interface, not Booster's high-level walk, stand-up, or kick interface.
  - MuJoCo must simulate the joint actuators or joint controller from the low-level commands; the simulator must not translate high-level motion modes into body movement.
  - MuJoCo must apply a Booster-compatible controller using commanded position, velocity, proportional and derivative gains, feed-forward torque, weight semantics, and physical actuator limits.
  - The exact control equation, clamping order, and behavior for invalid or incomplete commands must be documented and tested.
  - Valid commands must be applied in network arrival order; the MVP must not reorder them by source timestamp.
  - A malformed command must be rejected with a warning while retaining the previous valid command.
  - Joint names, signs, zero offsets, limits, and control modes must match the physical robot conventions.
- As a user, I want to connect the `hulk-ros-z` stack to any selected robot in the simulation without requiring physical hardware.
- As a user, I want the default Twix and `rosz` tooling to work with virtual robots using the same interfaces as for physical robots.
  - Tools must be able to discover and distinguish multiple virtual robots.

### Simulation Time and Determinism

- As a user, I want the MVP simulator and robotics stacks to run asynchronously and free-running.
  - Physics must advance using a fixed MuJoCo timestep independent of Bevy rendering frame rate.
  - Low-state data must be published at 500 Hz and camera images at 60 Hz using simulator timestamps.
  - Robotics outputs must be applied when received and remain active until superseded by a newer valid command.
  - A disconnected robot must immediately become physically frozen instead of continuing under its last command.
  - Freezing must fix every robot degree of freedom and clear its velocities while keeping collision geometry active so balls and other robots still collide with it.
- As a user, I want simulator time to be authoritative for physics, sensor timestamps, scenario events, and built-in message routing.
  - Pausing must freeze simulation time and all behavior derived from it.
  - Sensor schedules that do not share an integer period must retain their requested average cadence without being tied to the Bevy frame rate.
- As a user, I want to pause and resume the simulator.
  - Resuming must not execute catch-up steps accumulated from wall-clock time.
  - Sensor publication must stop while paused, but valid actuator commands may update the held joint targets without stepping physics.
  - The latest held targets must apply on the first physics step after resuming.
  - Object and parameter edits made while paused must take effect at a defined tick boundary.
- As a user, I want the MVP to run at maximum throughput and expose its current real-time factor.
  - Sensor rates are defined in simulation time and may therefore produce data faster than their nominal wall-clock rate.
  - Maximum-throughput execution must not wait for asynchronous robotics stacks to consume every sensor sample.
- As a user, I want simulator-owned computation to be deterministic for identical initial state, simulator inputs, software build, platform, and rendering backend.
  - Every stochastic simulator feature must use an explicit seed.
  - Physics, rendering inputs, scenario event ordering, and built-in message routing must be deterministic when given the same ordered actuator and user inputs.
  - Complete closed-loop full-stack runs are not required to be deterministic in the asynchronous MVP.

### Scenarios and World Setup

- As a user, I only want to simulate the number of robots, balls, and physical objects required by my scenario.
  - Objects and robots must have stable logical identities independent of their internal Bevy entity IDs.
  - Duplicate robot identities must be rejected with an actionable error.
  - Supported limits must be established from representative full-stack robot and camera benchmarks rather than assumed in advance.
- As a user, I want to add and remove robots, balls, and generic collidable obstacles while the simulation is running.
  - Runtime addition and removal of goals and lights is not required by the MVP.
- As a user, I want to add a virtual robot while the simulation is running.
  - Adding a robot must pause the simulation and must not affect existing robotics-stack processes or reset unrelated simulation state.
  - The new physical robot must appear immediately but remain frozen until a robotics stack connects and becomes ready.
  - The UI must show whether the robot is awaiting a connection, starting, ready, or under stack control.
  - The user may resume the rest of the simulation before a stack connects; an unconnected robot remains frozen.
  - A newly ready stack must take control of its robot atomically at a tick boundary without requiring another reset or manual start action.
- As a user, I want to write version-controlled Rust simulation scenarios outside the core simulator implementation.
  - The simulator must expose its runtime as a reusable library or Bevy plugin so scenario crates or binaries can configure it without modifying the core simulator.
  - Each scenario must be a small separate binary that constructs a Bevy `App`, adds the simulator plugin, and registers scenario systems.
  - Changing scenario code may rebuild and relink that scenario binary, but must not recompile unchanged core simulator dependencies.
  - Scenarios must define initial state and trigger events based on simulation time, robot positioning, goals, game state, or other world conditions.
  - Events must execute at deterministic tick boundaries in a documented order.
  - A condition that remains true must not repeatedly fire unless the event is explicitly configured to repeat.
- As a user, I want to reset simulator physics and scenario state without restarting or resetting connected external robotics stacks.
  - Existing stacks retain their internal state and continue controlling their assigned robots after the reset.
- As a user, I want to issue simulated robot button events from the UI or a scenario.
  - Scenarios must be able to automate the normal production startup button sequence so stacks can leave `Damping` without a simulation-only primary-state override.
- As a user, I want a built-in GameController that provides match state to all virtual robots through the production interface.
  - The GameController must support automatic and manual modes.
  - In manual mode, the user or a scenario must be able to control game state, whistles, goals, restarts, penalties, team setup, and match time.
  - Automatic mode must start in `Initial` and implement the core match flow through `Ready`, `Set`, and `Playing`, including goal detection, kickoffs, ball-out detection, and basic restarts.
  - Implemented automatic rules must follow the current HSL rules rather than simplified soccer behavior.
  - Automatic mode is continuous in the MVP: it does not run two halves, swap sides at halftime, or finish automatically from a match clock.
  - Penalties and rules outside the core match flow may be applied manually in the MVP.
  - A manual change made while automatic mode is active must temporarily override the current state without disabling subsequent automatic rule processing.
- As a user, I want the simulator to route GameController and team-communication messages deterministically between robotics stacks without using host UDP sockets.
  - Routing must preserve the production message types and per-robot namespaces.
  - MVP team communication must use immediate, lossless broadcast delivery within the sender's team.
  - MVP routing must not consume or enforce the GameController team-message budget.
  - Production-side message cooldowns remain active.
- As a user, I want the MVP to run a full match setup end to end.
  - Every simulated player on both teams must run an independent full-stack process.
  - The match must exercise simulated sensors and actuators, GameController state, team communication, perception, localization, behavior, and motion without ground-truth substitutions.
  - The required concurrent roster size will be set from explicit full-stack performance measurements rather than treated as an unmeasured correctness claim.

### Physics and Rendering

- As a user, I want robot cameras to be rendered efficiently through Bevy.
  - Multiple robots with multiple cameras must share rendering work where practical.
  - Camera rendering must be independent of the operator's 3D viewport.
  - Disabled cameras must not consume rendering work.
  - Target camera counts, resolutions, frame rates, and performance expectations must be defined by representative workloads.
- As a user, I want rendered cameras to use the intrinsics, extrinsics, stereo baseline, resolution, image format, and timestamps expected by the production camera pipeline.
  - Camera rate and resolution must be simulator parameters.
  - The default must be synchronized stereo images at 60 Hz using the full production-supported resolution.
  - The MVP must publish tightly packed production-compatible NV12 images; changing perception to consume RGB is outside the MVP.
  - Cameras must render to offscreen RGBA targets and use a shared GPU conversion stage to produce packed NV12 before asynchronous readback.
- As a user, I want the simulator to detect invalid numerical state, such as NaN values, solver divergence, or excessive velocities, and stop or pause with an actionable diagnostic instead of hanging.

### Interaction and Configuration

- As a remote user, I want Twix to show and control the current 3D ground-truth state of a simulator running on a headless host.
  - The simulator server must not require an operator window or display server.
  - The simulator must publish typed scene-state and control interfaces; Twix acts as a client rather than hosting simulation state.
  - The UI must clearly distinguish simulator truth from data perceived or estimated by a robot.
  - The UI must show simulation time, real-time factor, connected virtual robots, and stack readiness or failure state.
- As a simulator integrator, I want every interactive mutation exposed through a typed remote simulator API.
  - Pause and resume, entity addition and removal, pose edits, robot button events, simulator parameters, and GameController controls must not depend on an in-process UI.
- As a user, I want multiple clients to observe the same simulator while only one client owns mutation control.
  - Mutation requests from observer clients must be rejected.
  - Control ownership and the active controller must be visible to all clients.
- As a user, I want to select and move supported objects using gizmos while the simulation is paused.
  - I must be able to translate and rotate objects and enter an exact pose numerically.
  - I must be able to choose whether an edit preserves or clears object velocity.
  - Invalid placements must be rejected or clearly indicated.
  - UI edits are session-only; saving an interactively arranged world to a scenario file is not required by the MVP.
- As a user, I want to change parameters of a connected robotics stack at runtime through the normal parameter interfaces.
  - Parameter changes must target a specific virtual robot and preserve normal validation and revision semantics.
- As a user, I want to change simulator parameters such as field dimensions, ball properties, contact properties, and timestep.
  - Every parameter must define its unit, valid range, default, and whether it can change while running.
  - Runtime changes must be applied atomically at a defined tick boundary.
  - Structural changes must document which simulation state they preserve and which changes require a reset.
  - Session-only changes must not unexpectedly modify shared source-controlled parameter files.

### Performance and Reliability

- As a user, I want simulation performance to reduce development iteration time.
  - Performance must be measured using named workloads that specify robot count, active cameras, camera settings, physics timestep, and enabled robotics components.
  - Reported metrics must include simulated seconds per wall-clock second, physics-step time, sensor and rendering time, CPU and GPU utilization, and memory usage.
- As a CI user, I want to run scenarios without a window, display server, interactive viewer, or physical GPU unless rendered camera input is explicitly required.
- As a user, I want sensor streams to use bounded Zenoh QoS queues so slow or remote robotics stacks do not create unbounded latency or memory use.
  - Sensor streams must default to best-effort, keep-latest delivery so publication never blocks simulation progress.
  - Superseded sensor samples must be dropped.
  - Queue depth and drop counts must be observable per robot and sensor stream.
- As a user, I want every simulation run to have configurable limits for wall-clock time, simulation time, memory, and generated output so a faulty scenario cannot consume resources indefinitely.
- As a user, I want invalid setup data, unavailable ports, missing robotics components, and unsupported parameter combinations to produce actionable errors instead of panics or hangs.

## Future: Ground-Truth Substitution

Ground-truth substitution is explicitly outside the MVP. The full-stack integration should nevertheless use boundaries that allow individual computations to be replaced later without redesigning the simulator.

- As a user, I want to replace selected robotics computations with simulator ground truth so I can test downstream components in isolation.
  - Ground-truth interfaces must use versioned schemas with documented coordinate frames, units, timestamps, rates, and latency.
  - Enabling a substitution must disable or isolate conflicting upstream publishers.
  - Substitutions must be selectable per robot and independently for each supported computation.
- As a user, I want to provide object detections or features for visible objects directly and disable network inference.
- As a user, I want to provide the camera isometry between frames directly and disable visual odometry.
- As a user, I want to provide the relative ball position directly and disable the ball filter and its predecessors.
- As a user, I want to provide the correct robot pose directly and disable localization and its predecessors.

The purpose of these future substitutions is to let developers test components in isolation and iterate on behavior or motion while selected perception outputs are assumed to be correct.

## Future: Deterministic Full-Stack Execution

- As a user, I want physics, sensors, robotics timers, message delivery, and scenario events to run in deterministic lockstep using one logical clock.
- As a simulator integrator, I want a defined protocol between the simulator and each robotics stack's deterministic executor.
  - The executor must announce when all robotics nodes are instantiated and the stack is ready to accept sensor data.
  - For each logical sensor event, the protocol must identify the robot and logical time, return all topics produced while processing that event, and explicitly signal completion even when no topic was produced.
  - The executor may detect completion through graph quiescence internally, but graph-idle inference must not cross the simulator protocol boundary.
  - Low-state events must occur at 500 Hz and image events at 60 Hz without rounding either cadence to a common frame rate.
  - After publishing data for a logical event, the simulator must wait for every affected stack to signal completion before advancing time.
  - Commands returned for the same logical time must be applied atomically after all affected stacks complete.
  - If an executor produces no new actuator command, MuJoCo must retain the previous command.
- As a user, I want to advance a deterministic simulation by exactly one complete scheduled sensor event.

## Future: Automated Scenario Testing

- As a user, I want Rust scenarios to define assertions, completion conditions, and maximum simulation durations.
- As a user, I want headless scenarios to report success, assertion failure, timeout, invalid configuration, or robotics-stack failure through stable process exit behavior.
- As a user, I want to rerun a scenario with the same or a different random seed.

## Current Non-Goals

- Recording and replaying simulation runs.
- Generating reproduction or simulation manifests.
- Implementing or integrating the robotics stack's deterministic executor in the MVP.
- Saving interactively arranged worlds as scenario files.
