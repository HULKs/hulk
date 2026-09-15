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
    _node: Node,
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
    pub input_motion: MotionCommand,
    pub input_game: FilteredGameControllerState,
}

impl Robotics {
    pub async fn new(
        runtime: Handle,
        configuration: StackConfiguration,
        clock: Clock,
    ) -> Result<Self> {
        let context = Arc::new(
            ContextBuilder::default()
                .with_namespace(&configuration.namespace)
                .with_parameter_layers(configuration.parameter_layers.clone())
                .with_clock(clock.clone())
                .with_mode("client")
                .with_router_endpoint(&configuration.router)?
                .disable_multicast_scouting()
                .build()
                .await?,
        );
        let node = context.create_node("simulator_io").build().await?;
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
            "UI controls head; walking inference requests (0, 0, 0)".to_owned()
        } else {
            "External I/O only (robotics nodes disabled)".to_owned()
        });
        let ctx = context.clone();
        let launch = configuration.launch_nodes;
        let stack_task = runtime.spawn(async move {
            if !launch {
                return;
            }
            let mut tasks = JoinSet::new();
            tasks.spawn(crate::motion_dummy::run(ctx.clone()));
            tasks.spawn(motion::run_simulator_boxed(ctx.clone()));
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
            _node: node,
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
            input_motion: MotionCommand::Damping,
            input_game: FilteredGameControllerState::default(),
        })
    }

    pub fn status(&self) -> String {
        self.status.borrow().clone()
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
        // Wake node timers only after enqueuing observations for this physics step.
        self.clock.set_time(time)?;
        Ok(())
    }

    pub fn restart(&mut self) -> Result<()> {
        self.command_task.abort();
        self.stack_task.abort();
        self.runtime.block_on(async {
            let _ = (&mut self.command_task).await;
            let _ = (&mut self.stack_task).await;
        });
        self.context.shutdown()?;
        let mut replacement = self.runtime.block_on(Self::new(
            self.runtime.clone(),
            self.configuration.clone(),
            self.clock.clone(),
        ))?;
        replacement.input_motion = self.input_motion.clone();
        replacement.input_game = self.input_game.clone();
        *self = replacement;
        self.publish_inputs()
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
        exercise_head_motion(&runtime, &mut io, &clock);
        drop(io);
        server.shutdown().unwrap();
    }

    fn exercise_head_motion(runtime: &tokio::runtime::Runtime, io: &mut Robotics, clock: &Clock) {
        use crate::bevy_mujoco::{MjcfObject, MujocoWorld, MujocoWorldPlugin, SimulationMode};
        use bevy::prelude::*;

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
        let dimensions = FieldDimensions {
            width: 6.0,
            ..Default::default()
        };
        // Publish before the head node starts to exercise retained inputs.
        io.publish_field_dimensions(&dimensions).unwrap();
        let (mut tasks, head) = runtime.block_on(async {
            let mut tasks = JoinSet::new();
            tasks.spawn(publish_joint_limits(io.context.clone()));
            tasks.spawn(hardware_interface::run_boxed(io.context.clone()));
            tasks.spawn(crate::motion_dummy::run(io.context.clone()));
            tasks.spawn(motion::run_simulator_boxed(io.context.clone()));
            let head = tasks.spawn(head_motion::node::run_boxed(io.context.clone()));
            (tasks, head)
        });
        let mut step = |io: &mut Robotics, inferred: bool| {
            let observation = {
                let mut world = app.world_mut().resource_mut::<MujocoWorld>();
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
                let _ =
                    tokio::time::timeout(Duration::from_millis(100), io.commands.changed()).await;
            });
            if let Some(command) = io.latest_command() {
                assert_eq!(command.motor_commands.len(), 22);
                for (index, motor) in command.motor_commands.iter().enumerate().skip(2) {
                    let expected = match index {
                        3 => -78.0_f32.to_radians(),
                        5 => -30.0_f32.to_radians(),
                        7 => 78.0_f32.to_radians(),
                        9 => 30.0_f32.to_radians(),
                        10..=21 if inferred => 0.1,
                        _ => 0.0,
                    };
                    assert!((motor.position - expected).abs() < 1e-6);
                    assert!(motor.kp > 0.0 && motor.kd > 0.0);
                }
            }
            measured
        };
        io.input_motion = MotionCommand::Stand {
            head: HeadMotion::LookAround,
        };
        let mut maximum_yaw = 0.0_f32;
        let mut maximum_pitch = 0.0_f32;
        for _ in 0..250 {
            let head = step(io, false);
            maximum_yaw = maximum_yaw.max(head[0].position.abs());
            maximum_pitch = maximum_pitch.max(head[1].position);
        }
        assert!(maximum_yaw > 0.3, "scan did not move yaw: {maximum_yaw}");
        assert!(
            maximum_pitch > 0.3,
            "scan did not move pitch: {maximum_pitch}"
        );
        // The request remains unchanged throughout each phase, so this also
        // requires central motion to reevaluate it on simulation-clock ticks.
        io.input_motion = MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
        };
        for _ in 0..150 {
            step(io, false);
        }
        let measured = step(io, false);
        assert!(
            measured.iter().all(|motor| motor.position.abs() < 0.05),
            "ZeroAngles did not return the head to zero"
        );
        // No logical clock advance means neither commands nor the head move.
        runtime.block_on(async { tokio::time::sleep(Duration::from_millis(30)).await });
        io.commands.borrow_and_update();
        runtime.block_on(async { tokio::time::sleep(Duration::from_millis(30)).await });
        assert!(!io.commands.has_changed().unwrap());
        io.input_motion = MotionCommand::Damping;
        for _ in 0..3 {
            step(io, false);
        }
        assert_eq!(io.latest_command().unwrap().motor_commands[0].kp, 0.0);
        // A missing head service must not stop the dummy body commands or leave
        // the last active head target running.
        io.input_motion = MotionCommand::Stand {
            head: HeadMotion::LookAround,
        };
        for _ in 0..5 {
            step(io, false);
        }
        assert!(io.latest_command().unwrap().motor_commands[0].kp > 0.0);
        head.abort();
        for _ in 0..3 {
            step(io, false);
        }
        let command = io.latest_command().unwrap();
        assert_eq!(command.motor_commands[0].kp, 0.0);
        assert!(command.motor_commands[0].kd > 0.0);
        assert_eq!(command.motor_commands[10].kp, 80.0);
        // Exercise the walking service contract separately from ONNX execution:
        // requests must be exactly (0, 0, 0), and all leg command fields must
        // survive composition with independent arm and head commands.
        let walking = runtime.block_on(async {
            use motion_inference::node::{WALK_INFERENCE_SERVICE, WalkInferenceService};
            let node = io
                .context
                .create_node("walking_test")
                .build()
                .await
                .unwrap();
            let mut service = node
                .service_server::<WalkInferenceService>(WALK_INFERENCE_SERVICE)
                .build()
                .await
                .unwrap();
            tasks.spawn(async move {
                let _node = node;
                loop {
                    let (request, reply) = service.take_request_async().await?.into_parts();
                    assert_eq!(request.velocity.x(), 0.0);
                    assert_eq!(request.velocity.y(), 0.0);
                    assert_eq!(request.angular_velocity, 0.0);
                    let result: motion_inference::node::InferenceResult<_> = Ok(Box::new(
                        kinematics::joints::body::LowerBodyJoints::fill(booster::MotorCommand {
                            position: 0.1,
                            velocity: 0.2,
                            torque: 0.3,
                            kp: 12.0,
                            kd: 1.0,
                            ..Default::default()
                        }),
                    ));
                    reply.reply_async(&result).await?;
                }
            })
        });
        // Allow discovery and drain any in-flight fallback before checking legs.
        runtime.block_on(async { tokio::time::sleep(Duration::from_millis(30)).await });
        step(io, true);
        for _ in 0..3 {
            step(io, true);
        }
        for motor in &io.latest_command().unwrap().motor_commands[10..] {
            assert_eq!(motor.velocity, 0.2);
            assert_eq!(motor.torque, 0.3);
            assert_eq!(motor.kp, 12.0);
            assert_eq!(motor.kd, 1.0);
        }
        walking.abort();
        runtime.block_on(async { tokio::time::sleep(Duration::from_millis(30)).await });
        for _ in 0..3 {
            step(io, false);
        }
        runtime.block_on(async {
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
        });
    }
}
