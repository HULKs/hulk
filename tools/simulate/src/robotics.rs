//! The motion-only ROS-Z stack and the simulator's external topic boundary.
use std::{path::PathBuf, sync::Arc};

use bevy::prelude::Resource;
use booster::{LowCommand, LowState};
use color_eyre::{Result, eyre::eyre};
use coordinate_systems::{Ground, Robot};
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::{
    prelude::*,
    qos::{QosDurability, QosHistory},
    time::{Clock, Time},
};
use tokio::{
    runtime::Handle,
    sync::watch,
    task::{JoinHandle, JoinSet},
};
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    joint_limits::JointLimits, motion_command::MotionCommand, time_wrapper::TimeWrapper,
};

use crate::robot_io::{Observation, RobotBinding};

#[derive(Clone)]
pub struct StackConfiguration {
    pub router: String,
    pub namespace: String,
    pub parameter_layers: Vec<PathBuf>,
    pub launch_nodes: bool,
}

#[derive(Resource)]
pub struct Robotics {
    runtime: Handle,
    configuration: StackConfiguration,
    clock: Clock,
    context: Arc<Context>,
    parameter_overrides: Arc<tempfile::TempDir>,
    _node: Arc<Node>,
    pub parameters: crate::motion_parameters::ParameterClient,
    low_state: Publisher<LowState>,
    camera: Publisher<TimeWrapper<CameraMatrix>>,
    ground: Publisher<TimeWrapper<Option<Isometry3<Ground, Robot>>>>,
    motion: Publisher<MotionCommand>,
    game: Publisher<FilteredGameControllerState>,
    field: Publisher<FieldDimensions>,
    commands: watch::Receiver<Option<LowCommand>>,
    command_task: JoinHandle<()>,
    stack_task: JoinHandle<()>,
    status: watch::Receiver<String>,
    inference_status: watch::Receiver<Option<String>>,
    inference_status_task: JoinHandle<()>,
    pub input_motion: MotionCommand,
    pub input_game: FilteredGameControllerState,
}

impl Robotics {
    pub async fn new(
        runtime: Handle,
        configuration: StackConfiguration,
        clock: Clock,
    ) -> Result<Self> {
        Self::with_overrides(
            runtime,
            configuration,
            clock,
            Arc::new(tempfile::tempdir()?),
        )
        .await
    }

    async fn with_overrides(
        runtime: Handle,
        configuration: StackConfiguration,
        clock: Clock,
        parameter_overrides: Arc<tempfile::TempDir>,
    ) -> Result<Self> {
        let mut layers = configuration.parameter_layers.clone();
        layers.push(parameter_overrides.path().to_owned());
        let context = Arc::new(
            ContextBuilder::default()
                .with_namespace(&configuration.namespace)
                .with_parameter_layers(layers)
                .with_clock(clock.clone())
                .with_mode("client")
                .with_router_endpoint(&configuration.router)?
                .disable_multicast_scouting()
                .build()
                .await?,
        );
        let node = Arc::new(context.create_node("simulator_io").build().await?);
        let parameters = crate::motion_parameters::ParameterClient::start(
            &runtime,
            node.clone(),
            &configuration.namespace,
        );
        let latest = QosProfile {
            history: QosHistory::from_depth(1),
            ..Default::default()
        };
        let retained = QosProfile {
            durability: QosDurability::TransientLocal,
            ..latest
        };
        let low_state = node
            .publisher("inputs/low_state")
            .qos(latest)
            .build()
            .await?;
        let camera = node.publisher("camera_matrix").qos(latest).build().await?;
        let ground = node
            .publisher("ground_to_robot")
            .qos(latest)
            .build()
            .await?;
        let motion = node
            .publisher("behavior/motion_command")
            .qos(retained)
            .build()
            .await?;
        let game = node
            .publisher("filtered_game_controller_state")
            .qos(retained)
            .build()
            .await?;
        let field = node
            .publisher("field_dimensions")
            .qos(retained)
            .build()
            .await?;
        let sub = context
            .session()
            .declare_subscriber("rt/joint_ctrl")
            .await
            .map_err(|e| eyre!("{e}"))?;
        let (commands_tx, commands) = watch::channel(None);
        let command_task = runtime.spawn(async move {
            while let Ok(sample) = sub.recv_async().await {
                match cdr::deserialize::<LowCommand>(&sample.payload().to_bytes())
                    .map_err(|e| eyre!("{e}"))
                    .and_then(|command| {
                        RobotBinding::validate_command(&command)?;
                        Ok(command)
                    }) {
                    Ok(command) => {
                        commands_tx.send_replace(Some(command));
                    }
                    Err(error) => bevy::log::warn!("invalid rt/joint_ctrl command: {error:#}"),
                }
            }
        });
        let (status_tx, status) = watch::channel(if configuration.launch_nodes {
            "Main motion node controls body and head".to_owned()
        } else {
            "External I/O only (robotics nodes disabled)".to_owned()
        });
        let inference = node
            .subscriber::<motion_inference::node::Status>(motion_inference::node::STATUS_TOPIC)
            .qos(retained)
            .build()
            .await?;
        let (inference_tx, inference_status) = watch::channel(None);
        let inference_status_task = runtime.spawn(async move {
            while let Ok(status) = inference.recv().await {
                inference_tx.send_replace(match status.state {
                    motion_inference::node::State::Fault { reason } => Some(reason),
                    _ => None,
                });
            }
        });
        let ctx = context.clone();
        let launch = configuration.launch_nodes;
        let stack_task = runtime.spawn(async move {
            if !launch {
                return;
            }
            let mut tasks = JoinSet::new();
            tasks.spawn(forward_joint_commands(ctx.clone()));
            tasks.spawn(motion::run_boxed(ctx.clone()));
            tasks.spawn(publish_joint_limits(ctx.clone()));
            tasks.spawn(head_motion::node::run_boxed(ctx.clone()));
            tasks.spawn(motion_inference::run_boxed(ctx.clone()));
            tasks.spawn(hardware_interface::run_boxed(ctx));
            if let Some(result) = tasks.join_next().await {
                let reason = match result {
                    Ok(Ok(())) => "A motion-stack node exited".to_owned(),
                    Ok(Err(error)) => format!("Motion stack failed: {error:#}"),
                    Err(error) => format!("Motion stack task failed: {error}"),
                };
                status_tx.send_replace(reason);
                tasks.abort_all();
                while tasks.join_next().await.is_some() {}
            }
        });
        Ok(Self {
            runtime,
            configuration,
            clock,
            context,
            parameter_overrides,
            _node: node,
            parameters,
            low_state,
            camera,
            ground,
            motion,
            game,
            field,
            commands,
            command_task,
            stack_task,
            status,
            inference_status,
            inference_status_task,
            input_motion: MotionCommand::Damping,
            input_game: FilteredGameControllerState::default(),
        })
    }

    pub fn status(&self) -> String {
        self.inference_status.borrow().as_ref().map_or_else(
            || self.status.borrow().clone(),
            |reason| format!("Inference fault: {reason}"),
        )
    }

    pub fn latest_command(&self) -> Option<LowCommand> {
        self.commands.borrow().clone()
    }

    pub fn publish_inputs(&self) -> Result<()> {
        self.runtime.block_on(async {
            self.motion.publish(&self.input_motion).await?;
            self.game.publish(&self.input_game).await?;
            Ok(())
        })
    }

    pub fn publish_field_dimensions(&self, dimensions: &FieldDimensions) -> Result<()> {
        self.runtime.block_on(self.field.publish(dimensions))?;
        Ok(())
    }

    pub fn publish_observation(&self, observation: Observation, time: Time) -> Result<()> {
        // Service requests also run between timer ticks. Advance the shared clock
        // before exposing this frame so no consumer can see a future observation.
        self.clock.set_time(time)?;
        self.runtime.block_on(async {
            self.low_state
                .publish_with_source_time(&observation.low_state, time)
                .await?;
            self.camera
                .publish_with_source_time(
                    &TimeWrapper {
                        time,
                        inner: observation.camera_matrix,
                    },
                    time,
                )
                .await?;
            self.ground
                .publish_with_source_time(
                    &TimeWrapper {
                        time,
                        inner: Some(observation.ground_to_robot),
                    },
                    time,
                )
                .await?;
            Ok::<_, color_eyre::Report>(())
        })?;
        Ok(())
    }

    pub fn restart(&mut self) -> Result<()> {
        self.inference_status_task.abort();
        self.command_task.abort();
        self.stack_task.abort();
        self.runtime.block_on(async {
            let _ = (&mut self.command_task).await;
            let _ = (&mut self.stack_task).await;
        });
        self.context.shutdown()?;
        let mut replacement = self.runtime.block_on(Self::with_overrides(
            self.runtime.clone(),
            self.configuration.clone(),
            self.clock.clone(),
            self.parameter_overrides.clone(),
        ))?;
        replacement.input_motion = self.input_motion.clone();
        replacement.input_game = self.input_game.clone();
        *self = replacement;
        self.publish_inputs()
    }
}

// The upstream motion node publishes bare joints, whereas hardware_interface
// expects a motion envelope. Keep that transport adaptation in the simulator;
// all joint positions, velocities, torques and gains pass through unchanged.
async fn forward_joint_commands(context: Arc<Context>) -> Result<()> {
    use types::robot_command::{JointsCommand, MotionCommand as RobotCommand, MotionType};

    let node = context
        .create_node("simulator_joint_commands")
        .build()
        .await?;
    let joints = node
        .subscriber::<JointsCommand>("commands/joints_command")
        .build()
        .await?;
    let requests = node
        .subscriber::<MotionCommand>("behavior/motion_command")
        .cache(1)
        .build()
        .await?;
    let commands = node
        .publisher::<RobotCommand>("commands/motion_command")
        .build()
        .await?;
    loop {
        let joints_command = joints.recv().await?;
        let motion_type = match requests.get_latest().as_deref() {
            Some(MotionCommand::Prepare) => MotionType::Stand,
            Some(MotionCommand::Damping) | None => MotionType::Damping,
            _ => MotionType::Walk,
        };
        commands
            .publish(&RobotCommand {
                motion_type,
                joints_command,
            })
            .await?;
    }
}

async fn publish_joint_limits(context: Arc<Context>) -> Result<()> {
    let node = context
        .create_node("simulator_joint_limits")
        .build()
        .await?;
    let parameters = node.bind_parameter_as::<global_parameter_provider::Parameters>("global")?;
    parameters.add_validation_hook(|parameters| parameters.joint_limits.validate())?;
    let publisher = node
        .publisher::<JointLimits>("joint_limits")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let mut updates = parameters.subscribe();
    loop {
        let snapshot = updates.borrow_and_update().clone();
        publisher.publish(&snapshot.typed().joint_limits).await?;
        updates.changed().await?;
    }
}

impl Drop for Robotics {
    fn drop(&mut self) {
        self.inference_status_task.abort();
        self.command_task.abort();
        self.stack_task.abort();
        let _ = self.context.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use booster::MotorState;
    use std::time::Duration;
    use types::motion_command::HeadMotion;

    #[test]
    fn external_topics_preserve_types_source_time_and_raw_command_encoding() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let router = format!("tcp/127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);
        let clock = Clock::logical(Time::zero());
        let (server, mut io, sensors, camera, ground, motion, game) = runtime.block_on(async {
            let server = ContextBuilder::default()
                .with_namespace("/simulator")
                .with_mode("router")
                .disable_multicast_scouting()
                .with_connect_endpoints(std::iter::empty::<&str>())
                .with_listen_endpoints([router.as_str()])
                .build()
                .await
                .unwrap();
            let io = Robotics::new(
                runtime.handle().clone(),
                StackConfiguration {
                    router,
                    namespace: "/simulator/test_robot".into(),
                    parameter_layers: vec![
                        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("parameters"),
                        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../etc/parameters/base"),
                    ],
                    launch_nodes: false,
                },
                clock.clone(),
            )
            .await
            .unwrap();
            let sensors = io
                ._node
                .subscriber::<LowState>("inputs/low_state")
                .build()
                .await
                .unwrap();
            let camera = io
                ._node
                .subscriber::<TimeWrapper<CameraMatrix>>("camera_matrix")
                .build()
                .await
                .unwrap();
            let ground = io
                ._node
                .subscriber::<TimeWrapper<Option<Isometry3<Ground, Robot>>>>("ground_to_robot")
                .build()
                .await
                .unwrap();
            let motion = io
                ._node
                .subscriber::<MotionCommand>("behavior/motion_command")
                .build()
                .await
                .unwrap();
            let game = io
                ._node
                .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
                .build()
                .await
                .unwrap();
            (server, io, sensors, camera, ground, motion, game)
        });
        let low_state = LowState {
            motor_state_serial: vec![MotorState::default(); 22],
            ..Default::default()
        };
        let time = Time::from_nanos(2_000_000);
        io.publish_observation(
            Observation {
                low_state,
                camera_matrix: CameraMatrix::default(),
                ground_to_robot: Isometry3::identity(),
            },
            time,
        )
        .unwrap();
        io.input_motion = MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
        };
        io.input_game.remaining_number_of_messages = 23;
        io.publish_inputs().unwrap();
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), async {
                let sensor = sensors.recv_with_metadata().await.unwrap();
                assert_eq!(sensor.source_time, time);
                assert_eq!(sensor.message.motor_state_serial.len(), 22);
                let camera = camera.recv_with_metadata().await.unwrap();
                assert_eq!(camera.source_time, time);
                assert_eq!(camera.message.time, time);
                let ground = ground.recv_with_metadata().await.unwrap();
                assert_eq!(ground.source_time, time);
                assert!(ground.message.inner.is_some());
                assert_eq!(motion.recv().await.unwrap(), io.input_motion);
                assert_eq!(game.recv().await.unwrap().remaining_number_of_messages, 23);
                let command = LowCommand {
                    command_type: booster::CommandType::Serial,
                    motor_commands: vec![
                        booster::MotorCommand {
                            kp: 8.0,
                            kd: 1.0,
                            position: 0.2,
                            ..Default::default()
                        };
                        22
                    ],
                };
                let payload = cdr::serialize::<_, _, cdr::CdrLe>(&command, cdr::Infinite).unwrap();
                io.context
                    .session()
                    .put("rt/joint_ctrl", payload)
                    .await
                    .unwrap();
                io.commands.changed().await.unwrap();
                assert_eq!(io.latest_command().unwrap().motor_commands[0].kp, 8.0);
            })
            .await
            .expect("external topic delivery timed out");
        });
        assert_eq!(clock.now(), time);
        io.restart().unwrap();
        assert_eq!(clock.now(), time, "restarting must not rewind MuJoCo time");
        assert!(
            io.latest_command().is_none(),
            "old commands must not survive reset"
        );
        assert_eq!(
            io.input_motion,
            MotionCommand::Stand {
                head: HeadMotion::ZeroAngles
            }
        );
        exercise_observation_clock_ordering(&runtime, &io, &clock);
        exercise_head_motion(&runtime, &mut io, &clock);
        drop(io);
        server.shutdown().unwrap();
    }

    fn exercise_observation_clock_ordering(
        runtime: &tokio::runtime::Runtime,
        io: &Robotics,
        clock: &Clock,
    ) {
        let subscriber = runtime.block_on(async {
            io._node
                .subscriber::<LowState>("inputs/low_state")
                .build()
                .await
                .unwrap()
        });
        let observation = || Observation {
            low_state: LowState {
                motor_state_serial: vec![MotorState::default(); 22],
                ..Default::default()
            },
            camera_matrix: CameraMatrix::default(),
            ground_to_robot: Isometry3::identity(),
        };
        let (observed_tx, mut observed_rx) = tokio::sync::mpsc::unbounded_channel();
        let observed_clock = clock.clone();
        let receiver = runtime.spawn(async move {
            while let Ok(sample) = subscriber.recv_with_metadata().await {
                let now = observed_clock.now();
                observed_tx.send((sample.source_time, now)).unwrap();
            }
        });
        // Check inside a concurrent consumer, not after publish_observation returns:
        // a service request may inspect this sample while publication is still running.
        for _ in 0..512 {
            let time = clock.now() + Duration::from_millis(2);
            io.publish_observation(observation(), time).unwrap();
            let (source_time, observed_time) = runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(2), observed_rx.recv())
                    .await
                    .unwrap()
                    .unwrap()
            });
            assert_eq!(source_time, time);
            assert!(
                source_time <= observed_time,
                "sensor timestamp {source_time:?} is ahead of receiver clock {observed_time:?}"
            );
        }
        receiver.abort();
        runtime.block_on(async {
            let _ = receiver.await;
        });
        let subscriber = runtime.block_on(async {
            io._node
                .subscriber::<LowState>("inputs/low_state")
                .build()
                .await
                .unwrap()
        });
        // A rejected timestamp must not leak a sensor frame to running nodes.
        assert!(io.publish_observation(observation(), Time::zero()).is_err());
        assert!(
            !subscriber.is_ready(),
            "rejected clock update published a sensor frame"
        );
    }

    fn exercise_head_motion(runtime: &tokio::runtime::Runtime, io: &mut Robotics, clock: &Clock) {
        use crate::bevy_mujoco::{MjcfObject, MujocoWorld, MujocoWorldPlugin, SimulationMode};
        use bevy::prelude::*;
        use kinematics::joints::{Joints, body::LowerBodyJoints};
        use motion_inference::node::{
            GETUP_INFERENCE_SERVICE, GetUpInferenceService, InferenceResult,
            KICK_INFERENCE_SERVICE, KickInferenceService, WALK_INFERENCE_SERVICE,
            WalkInferenceService,
        };
        use types::robot_command::MotorCommand;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, MujocoWorldPlugin));
        app.insert_resource(SimulationMode::Paused);
        let robot = app
            .world_mut()
            .spawn((
                MjcfObject::new(
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/k1_robot.xml"),
                    "Trunk",
                )
                .with_free_joint("world_joint")
                .grounded(),
                Transform::default(),
            ))
            .id();
        app.update();
        let binding = RobotBinding::new(
            app.world().resource::<MujocoWorld>().data(),
            &format!("object_{}_", robot.to_bits()),
        )
        .unwrap();
        io.publish_field_dimensions(&FieldDimensions {
            width: 6.0,
            ..Default::default()
        })
        .unwrap();
        let (walk_tx, mut walks) = tokio::sync::mpsc::unbounded_channel();
        let (kick_tx, mut kicks) = tokio::sync::mpsc::unbounded_channel();
        let (getup_tx, mut getups) = tokio::sync::mpsc::unbounded_channel();
        let (mut tasks, inference_node) = runtime.block_on(async {
            let mut tasks = JoinSet::new();
            let node = Arc::new(
                io.context
                    .create_node("motion_inference")
                    .build()
                    .await
                    .unwrap(),
            );
            let mut walk = node
                .service_server::<WalkInferenceService>(WALK_INFERENCE_SERVICE)
                .build()
                .await
                .unwrap();
            let mut kick = node
                .service_server::<KickInferenceService>(KICK_INFERENCE_SERVICE)
                .build()
                .await
                .unwrap();
            let mut getup = node
                .service_server::<GetUpInferenceService>(GETUP_INFERENCE_SERVICE)
                .build()
                .await
                .unwrap();
            tasks.spawn(async move {
                loop {
                    let (request, reply) = walk.take_request_async().await?.into_parts();
                    walk_tx.send(request).unwrap();
                    let result: InferenceResult<_> =
                        Ok(Box::new(LowerBodyJoints::fill(MotorCommand {
                            position: request.velocity.x(),
                            velocity: 0.2,
                            torque: 0.3,
                            kp: 80.0,
                            kd: 4.0,
                        })));
                    reply.reply_async(&result).await?;
                }
            });
            tasks.spawn(async move {
                loop {
                    let (request, reply) = kick.take_request_async().await?.into_parts();
                    kick_tx.send(request).unwrap();
                    let result: InferenceResult<_> =
                        Ok(Box::new(LowerBodyJoints::fill(MotorCommand {
                            position: 0.12,
                            kp: 22.0,
                            ..MotorCommand::zeros()
                        })));
                    reply.reply_async(&result).await?;
                }
            });
            tasks.spawn(async move {
                loop {
                    let (request, reply) = getup.take_request_async().await?.into_parts();
                    getup_tx.send(request).unwrap();
                    let result: InferenceResult<_> = Ok(Box::new(Joints::fill(MotorCommand {
                        position: 0.03,
                        kp: 30.0,
                        ..MotorCommand::zeros()
                    })));
                    reply.reply_async(&result).await?;
                }
            });
            tasks.spawn(publish_joint_limits(io.context.clone()));
            tasks.spawn(hardware_interface::run_boxed(io.context.clone()));
            tasks.spawn(forward_joint_commands(io.context.clone()));
            tasks.spawn(motion::run_boxed(io.context.clone()));
            tasks.spawn(head_motion::node::run_boxed(io.context.clone()));
            (tasks, node)
        });
        let mut step = |io: &mut Robotics| {
            let observation = {
                let mut world = app.world_mut().resource_mut::<MujocoWorld>();
                // The service stub tests routing, not balance. Support the torso
                // above the floor while exercising real head-joint physics.
                world
                    .set_object_pose(robot, Transform::from_xyz(0.0, 1.0, 0.0))
                    .unwrap();
                let data = world.data_mut();
                for _ in 0..10 {
                    binding.apply(data, io.latest_command().as_ref());
                    data.step();
                }
                data.forward();
                binding.observe(data)
            };
            let measured = observation.low_state.motor_state_serial[0..2].to_vec();
            io.publish_inputs().unwrap();
            io.publish_observation(observation, clock.now() + Duration::from_millis(20))
                .unwrap();
            runtime.block_on(async {
                // Nodes can finish startup after this logical tick. Let the next
                // tick drive them; the assertions below require actual outputs.
                let _ =
                    tokio::time::timeout(Duration::from_millis(100), io.commands.changed()).await;
                if let Some(result) = tasks.try_join_next() {
                    panic!("motion stack task exited: {result:?}");
                }
            });
            measured
        };
        io.input_motion = MotionCommand::Stand {
            head: HeadMotion::LookAround,
        };
        let mut maximum_yaw = 0.0_f32;
        let mut maximum_pitch = 0.0_f32;
        for _ in 0..250 {
            let head = step(io);
            maximum_yaw = maximum_yaw.max(head[0].position.abs());
            maximum_pitch = maximum_pitch.max(head[1].position);
        }
        assert!(maximum_yaw > 0.3, "scan did not move yaw: {maximum_yaw}");
        assert!(
            maximum_pitch > 0.3,
            "scan did not move pitch: {maximum_pitch}"
        );
        while let Ok(request) = walks.try_recv() {
            assert_eq!(request.velocity, linear_algebra::Vector2::zeros());
            assert_eq!(request.angular_velocity, 0.0);
        }
        io.input_motion = MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
        };
        for _ in 0..150 {
            step(io);
        }
        let measured = step(io);
        assert!(
            measured.iter().all(|motor| motor.position.abs() < 0.05),
            "head did not return to zero: {:?}",
            measured
                .iter()
                .map(|motor| motor.position)
                .collect::<Vec<_>>()
        );
        runtime.block_on(async { tokio::time::sleep(Duration::from_millis(30)).await });
        io.commands.borrow_and_update();
        runtime.block_on(async { tokio::time::sleep(Duration::from_millis(30)).await });
        assert!(
            !io.commands.has_changed().unwrap(),
            "commands must stop with logical time"
        );

        // Real central motion must forward UI velocities instead of forcing zero walking.
        while walks.try_recv().is_ok() {}
        io.input_motion = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: linear_algebra::vector![0.25, -0.1],
            angular_velocity: 0.4,
        };
        for _ in 0..5 {
            step(io);
        }
        let mut last = None;
        while let Ok(request) = walks.try_recv() {
            last = Some(request);
        }
        let request = last.unwrap();
        assert_eq!(request.velocity, linear_algebra::vector![0.25, -0.1]);
        assert_eq!(request.angular_velocity, 0.4);
        let command = io.latest_command().unwrap();
        for motor in &command.motor_commands[10..] {
            assert_eq!(motor.position, 0.25);
            assert_eq!(motor.velocity, 0.2);
            assert_eq!(motor.torque, 0.3);
            assert_eq!(motor.kp, 80.0);
        }
        // Preserve the upstream walking arm controller's configured gains.
        assert!(
            command.motor_commands[2..10]
                .iter()
                .all(|motor| motor.position.is_finite()
                    && motor.velocity == 0.0
                    && motor.torque == 0.0
                    && motor.kp == 40.0
                    && motor.kd == 1.0)
        );

        io.input_motion = MotionCommand::VisualKick {
            head: HeadMotion::ZeroAngles,
            ball_position: linear_algebra::point![0.2, -0.1],
            kick_direction: linear_algebra::Orientation2::new(0.3),
            target_position: linear_algebra::point![2.0, 0.0],
            robot_theta_to_field: linear_algebra::Orientation2::identity(),
            kick_power: types::motion_command::KickPower::Schlong,
        };
        for _ in 0..5 {
            step(io);
        }
        let kick = kicks.try_recv().unwrap();
        assert!(kick.request.strong);
        assert_eq!(
            kick.request.ball_position,
            linear_algebra::point![0.2, -0.1]
        );
        assert!((kick.request.direction - 0.3).abs() < 1e-6);
        assert_eq!(io.latest_command().unwrap().motor_commands[10].kp, 22.0);
        io.input_motion = MotionCommand::StandUp;
        for _ in 0..5 {
            step(io);
        }
        assert!(!getups.try_recv().unwrap().fast);
        assert!(
            io.latest_command()
                .unwrap()
                .motor_commands
                .iter()
                .all(|motor| motor.kp == 30.0 && motor.position == 0.03)
        );

        io.input_motion = MotionCommand::Damping;
        for _ in 0..3 {
            step(io);
        }
        assert!(
            io.latest_command()
                .unwrap()
                .motor_commands
                .iter()
                .all(|motor| motor.kp == 0.0 && motor.kd == 0.0)
        );
        runtime.block_on(async {
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
        });
        drop(inference_node);
    }
}
