//! The behavior and motion ROS-Z stack and the simulator's external topic boundary.
use std::{path::PathBuf, sync::Arc};

use bevy::prelude::Resource;
use booster::{LowCommand, LowState};
use color_eyre::{Result, eyre::eyre};
use coordinate_systems::{Ground, Robot};
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::{
    parameter::RemoteParameterClient,
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
    motion_command::MotionCommand, time_wrapper::TimeWrapper,
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
    behavior_inputs: crate::behavior_inputs::BehaviorInputs,
    injection: watch::Sender<Option<MotionCommand>>,
    injection_task: JoinHandle<()>,
    injection_status: watch::Receiver<String>,
    motion: ros_z::cache::Cache<MotionCommand>,
    execution: ros_z::cache::Cache<types::motion_execution::MotionExecution>,
    game: Publisher<FilteredGameControllerState>,
    field: Option<Publisher<FieldDimensions>>,
    field_updates: watch::Sender<Option<FieldDimensions>>,
    commands: watch::Receiver<Option<LowCommand>>,
    command_task: JoinHandle<()>,
    stack_task: JoinHandle<()>,
    status: watch::Receiver<String>,
    inference_status: watch::Receiver<Option<String>>,
    inference_status_task: JoinHandle<()>,
    pub input_motion: MotionCommand,
    pub injection_enabled: bool,
    pub input_game: FilteredGameControllerState,
}

impl Robotics {
    pub async fn new(
        runtime: Handle,
        configuration: StackConfiguration,
        clock: Clock,
    ) -> Result<Self> {
        let overrides = Arc::new(tempfile::tempdir()?);
        std::fs::create_dir(overrides.path().join("live"))?;
        // Start under behavior control, independent of robot parameter injections.
        std::fs::write(
            overrides.path().join("behavior_node.json5"),
            r#"{control: {injected_motion_command: null, remote_control: {enable: false}}}"#,
        )?;
        Self::with_overrides(runtime, configuration, clock, overrides).await
    }

    async fn with_overrides(
        runtime: Handle,
        configuration: StackConfiguration,
        clock: Clock,
        parameter_overrides: Arc<tempfile::TempDir>,
    ) -> Result<Self> {
        let mut layers = configuration.parameter_layers.clone();
        layers.push(parameter_overrides.path().to_owned());
        // Keep the null injection in its own lower layer: recursive parameter
        // merging otherwise combines the base enum variant with UI variants.
        layers.push(parameter_overrides.path().join("live"));
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
            .subscriber("behavior/motion_command")
            .cache(1)
            .build()
            .await?;
        let execution = node
            .subscriber(types::motion_execution::MOTION_EXECUTION_TOPIC)
            .qos(QosProfile {
                reliability: ros_z::qos::QosReliability::BestEffort,
                ..latest
            })
            .cache(1)
            .build()
            .await?;
        let behavior_inputs = crate::behavior_inputs::BehaviorInputs::new(&node).await?;
        let client = RemoteParameterClient::new(
            node.clone(),
            format!(
                "{}/behavior_node",
                configuration.namespace.trim_end_matches('/')
            ),
        )?;
        let (injection, updates) = watch::channel(None);
        let (injection_status_tx, injection_status) =
            watch::channel("Behavior controls motion".into());
        let injection_task =
            runtime.spawn(synchronize_injection(client, updates, injection_status_tx));
        let game = node
            .publisher("filtered_game_controller_state")
            .qos(retained)
            .build()
            .await?;
        let field = if configuration.launch_nodes {
            None
        } else {
            Some(
                node.publisher("field_dimensions")
                    .qos(retained)
                    .build()
                    .await?,
            )
        };
        let (field_updates, field_receiver) = watch::channel(None);
        let global_parameters = RemoteParameterClient::new(
            node.clone(),
            format!(
                "{}/global_parameter_provider",
                configuration.namespace.trim_end_matches('/')
            ),
        )?;
        let global_fields = node
            .subscriber::<FieldDimensions>("field_dimensions")
            .qos(retained)
            .build()
            .await?;
        let field_layer = parameter_overrides
            .path()
            .join("live")
            .to_string_lossy()
            .into_owned();
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
            "Behavior and motion nodes running".to_owned()
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
            tasks.spawn(crate::simulated_sdk::run(ctx.clone()));
            tasks.spawn(behavior_node::node::run_boxed(ctx.clone()));
            tasks.spawn(fall_detection::run_boxed(ctx.clone()));
            tasks.spawn(ball_state_composer::run_boxed(ctx.clone()));
            tasks.spawn(rule_obstacle_composer::run_boxed(ctx.clone()));
            tasks.spawn(motion::run_boxed(ctx.clone()));
            tasks.spawn(global_parameter_provider::run_boxed(ctx.clone()));
            tasks.spawn(synchronize_field_dimensions(
                global_parameters,
                global_fields,
                field_receiver,
                field_layer,
            ));
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
            execution,
            behavior_inputs,
            injection,
            injection_task,
            injection_status,
            game,
            field,
            field_updates,
            commands,
            command_task,
            stack_task,
            status,
            inference_status,
            inference_status_task,
            input_motion: MotionCommand::Damping,
            injection_enabled: false,
            input_game: FilteredGameControllerState::default(),
        })
    }

    pub fn status(&self) -> String {
        if let Some(execution) = self.execution.get_latest()
            && let Some(reason) = &execution.fault
        {
            return format!("Motion fault: {reason}");
        }
        self.inference_status.borrow().as_ref().map_or_else(
            || {
                format!(
                    "{} · {}",
                    self.status.borrow().as_str(),
                    self.injection_status.borrow().as_str()
                )
            },
            |reason| format!("Inference fault: {reason}"),
        )
    }

    pub fn latest_command(&self) -> Option<LowCommand> {
        self.commands.borrow().clone()
    }

    pub fn active_motion(&self) -> MotionCommand {
        self.motion
            .get_latest()
            .map(|motion| motion.as_ref().clone())
            .unwrap_or_default()
    }

    pub fn inject_current_motion(&mut self) -> Result<()> {
        self.injection_enabled = true;
        // An explicit Send also overrides a Twix edit to the same previously sent command.
        self.injection.send_replace(Some(self.input_motion.clone()));
        self.publish_inputs()
    }

    pub fn clear_injection(&mut self) -> Result<()> {
        self.injection_enabled = false;
        self.injection.send_replace(None);
        self.publish_inputs()
    }

    pub fn publish_inputs(&self) -> Result<()> {
        let next = self.injection_enabled.then(|| self.input_motion.clone());
        self.injection.send_if_modified(|current| {
            if *current == next {
                return false;
            }
            *current = next;
            true
        });
        self.runtime.block_on(async {
            self.behavior_inputs.publish_game(&self.input_game).await?;
            self.game.publish(&self.input_game).await?;
            Ok(())
        })
    }

    pub fn publish_world(
        &self,
        ground: nalgebra::Isometry3<f32>,
        ball: Option<([f64; 3], [f64; 3])>,
        obstacles: Vec<[f64; 3]>,
        time: Time,
    ) -> Result<()> {
        self.runtime.block_on(self.behavior_inputs.publish(
            ground,
            ball,
            obstacles,
            self.input_game.global_field_side,
            time,
        ))
    }

    pub fn publish_field_dimensions(&self, dimensions: &FieldDimensions) -> Result<()> {
        if let Some(field) = &self.field {
            self.runtime.block_on(field.publish(dimensions))?;
        } else {
            self.field_updates.send_replace(Some(*dimensions));
        }
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
        self.injection_task.abort();
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
        replacement.injection_enabled = self.injection_enabled;
        replacement.input_game = self.input_game.clone();
        // Reapply even a pending clear: reset may interrupt its parameter RPC.
        replacement.injection.send_replace(
            replacement
                .injection_enabled
                .then(|| replacement.input_motion.clone()),
        );
        *self = replacement;
        self.publish_inputs()
    }
}

async fn synchronize_injection(
    client: RemoteParameterClient,
    mut updates: watch::Receiver<Option<MotionCommand>>,
    status: watch::Sender<String>,
) {
    // Only write on a UI change; periodic game publication must not overwrite Twix edits.
    while updates.changed().await.is_ok() {
        loop {
            let command = updates.borrow_and_update().clone();
            status.send_replace("Applying motion override...".into());
            let result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
                let snapshot = client.get_snapshot().await?;
                let layer = snapshot
                    .layers
                    .last()
                    .ok_or_else(|| eyre!("Behavior node has no writable layer"))?;
                Ok::<_, color_eyre::Report>(
                    client
                        .set_json(
                            "control.injected_motion_command",
                            &serde_json::to_value(&command)?,
                            layer.clone(),
                            None,
                        )
                        .await?,
                )
            })
            .await;
            match result {
                Ok(Ok(response)) if response.success => {
                    status.send_replace(
                        if command.is_some() {
                            "UI motion injected"
                        } else {
                            "Behavior controls motion"
                        }
                        .into(),
                    );
                    break;
                }
                result => {
                    status.send_replace(format!("Motion override pending: {result:?}"));
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            }
        }
    }
}

async fn synchronize_field_dimensions(
    parameters: RemoteParameterClient,
    fields: Subscriber<FieldDimensions>,
    mut updates: watch::Receiver<Option<FieldDimensions>>,
    layer: String,
) -> Result<()> {
    // The provider publishes its initial field after registering its parameter services.
    fields.recv().await?;
    loop {
        let dimensions = *updates.borrow_and_update();
        if let Some(dimensions) = dimensions {
            let response = parameters
                .set_json(
                    "field_dimensions",
                    &serde_json::to_value(dimensions)?,
                    layer.clone(),
                    None,
                )
                .await?;
            if !response.success {
                return Err(eyre!(
                    "Cannot update global field dimensions: {}",
                    response.message
                ));
            }
        }
        updates.changed().await?;
    }
}

impl Drop for Robotics {
    fn drop(&mut self) {
        self.injection_task.abort();
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
    #[ignore = "requires ORT_DYLIB_PATH and downloaded K1 models"]
    fn real_stack_drives_simulated_robot_after_startup() {
        use crate::bevy_mujoco::{MjcfObject, MujocoWorld, MujocoWorldPlugin, SimulationMode};
        use bevy::prelude::*;
        use ros_z::time::Time;
        use types::{
            fall_detection::{FALL_DETECTION_TOPIC, FallDetection},
            hardware_status::{HARDWARE_STATUS_TOPIC, HardwareStatus},
        };
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let router = format!("tcp/127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let layer = tempfile::tempdir().unwrap();
        std::fs::write(
            layer.path().join("motion_inference.json5"),
            serde_json::json!({"neural_networks_folder": root.join("etc/neural_networks")})
                .to_string(),
        )
        .unwrap();
        let clock = Clock::logical(Time::zero());
        let (server, mut io, fall, hardware, inference) = runtime.block_on(async {
            let server = ContextBuilder::default()
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
                    namespace: "/real_stack_test".into(),
                    parameter_layers: vec![
                        root.join("tools/simulate/parameters"),
                        root.join("etc/parameters/base"),
                        root.join("etc/parameters/location/simulator"),
                        layer.path().to_owned(),
                    ],
                    launch_nodes: true,
                },
                clock.clone(),
            )
            .await
            .unwrap();
            let qos = QosProfile {
                reliability: ros_z::qos::QosReliability::BestEffort,
                ..Default::default()
            };
            let fall = io
                ._node
                .subscriber::<FallDetection>(FALL_DETECTION_TOPIC)
                .qos(qos)
                .cache(1)
                .build()
                .await
                .unwrap();
            let hardware = io
                ._node
                .subscriber::<HardwareStatus>(HARDWARE_STATUS_TOPIC)
                .qos(qos)
                .cache(1)
                .build()
                .await
                .unwrap();
            let inference = io
                ._node
                .subscriber::<motion_inference::node::Status>(motion_inference::node::STATUS_TOPIC)
                .qos(QosProfile {
                    durability: QosDurability::TransientLocal,
                    ..qos
                })
                .cache(1)
                .build()
                .await
                .unwrap();
            (server, io, fall, hardware, inference)
        });
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, MujocoWorldPlugin));
        app.insert_resource(SimulationMode::Paused);
        let robot = app
            .world_mut()
            .spawn((
                MjcfObject::new(root.join("tools/simulate/assets/k1_robot.xml"), "Trunk")
                    .with_free_joint("world_joint")
                    .grounded(),
                Transform::default(),
            ))
            .id();
        app.update();
        let mut world = app.world_mut().resource_mut::<MujocoWorld>();
        let binding =
            RobotBinding::new(world.data(), &format!("object_{}_", robot.to_bits())).unwrap();
        io.publish_field_dimensions(&FieldDimensions::SPL_2025)
            .unwrap();
        io.publish_observation(binding.observe(world.data()), Time::zero())
            .unwrap();
        io.publish_inputs().unwrap();
        // Load real policies while paused: Idle and Initialized have the same
        // source timestamp. The old motion subscriber stayed Idle forever here.
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    if let Some(status) = inference.get_latest() {
                        match &status.state {
                            motion_inference::node::State::Initialized => break,
                            motion_inference::node::State::Fault { reason } => panic!("{reason}"),
                            _ => {}
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("real inference did not initialize");
        });
        assert_eq!(clock.now(), Time::zero());
        let mut driven = 0;
        let mut maximum_head_yaw = 0.0_f32;
        let mut walk_start_x = 0.0;
        let mut walk_distance = 0.0;
        let walk = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: linear_algebra::vector![0.15, 0.0],
            angular_velocity: 0.0,
        };
        for frame in 0..375 {
            if frame == 125 {
                walk_start_x = binding.ground_to_world(world.data()).translation.x;
                io.input_motion = walk.clone();
                io.inject_current_motion().unwrap();
            }
            if frame == 250 {
                assert_eq!(io.active_motion(), walk);
                walk_distance = binding.ground_to_world(world.data()).translation.x - walk_start_x;
                io.clear_injection().unwrap();
            }
            // Match Bevy's catch-up batches at roughly 60 rendered frames per second.
            for _ in 0..8 {
                binding.apply(world.data_mut(), io.latest_command().as_ref());
                world.data_mut().step();
                world.data_mut().forward();
                let time = Time::from_nanos((world.data().time() * 1e9).round() as i64);
                io.publish_observation(binding.observe(world.data()), time)
                    .unwrap();
                io.publish_world(binding.ground_to_world(world.data()), None, vec![], time)
                    .unwrap();
                io.publish_inputs().unwrap();
                driven += usize::from(
                    io.latest_command()
                        .is_some_and(|c| c.motor_commands.iter().any(|m| m.kp > 0.0)),
                );
            }
            runtime.block_on(async { tokio::time::sleep(Duration::from_millis(16)).await });
            if frame < 125 {
                maximum_head_yaw = maximum_head_yaw.max(
                    binding.observe(world.data()).low_state.motor_state_serial[0]
                        .position
                        .abs(),
                );
            }
            assert!(
                io.execution.get_latest().is_none_or(|e| e.fault.is_none()),
                "{}",
                io.status()
            );
        }

        assert!(
            driven > 100,
            "no sustained joint actuation: {}",
            io.status()
        );
        assert!(
            maximum_head_yaw > 0.3,
            "behavior did not drive the physical head: {maximum_head_yaw}"
        );
        assert!(
            walk_distance > 0.05,
            "injected walking did not move the robot: {walk_distance}"
        );
        assert!(
            matches!(io.active_motion(), MotionCommand::Stand { .. }),
            "behavior did not resume: {:?}",
            io.active_motion()
        );
        assert!(fall.get_latest().unwrap().is_upright(clock.now()));
        let hardware = hardware.get_latest().unwrap();
        assert_eq!(
            hardware.acknowledged,
            Some(types::hardware_status::ControlMode::Custom)
        );
        assert!(hardware.fault.is_none(), "{:?}", hardware.fault);
        eprintln!(
            "Measured head yaw {maximum_head_yaw:.3} rad; injected walking distance {walk_distance:.3} m; autonomous Stand resumed"
        );
        drop(io);
        server.shutdown().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn global_parameters_publish_simulator_layer_and_live_field_updates() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../etc/parameters");
        let overrides = tempfile::tempdir().unwrap();
        let context = Arc::new(
            ContextBuilder::default()
                .with_namespace("/global_parameters_test")
                .with_mode("peer")
                .disable_multicast_scouting()
                .with_connect_endpoints(std::iter::empty::<&str>())
                .with_listen_endpoints(std::iter::empty::<&str>())
                .with_parameter_layers([
                    root.join("base"),
                    root.join("location/simulator"),
                    overrides.path().to_owned(),
                ])
                .build()
                .await
                .unwrap(),
        );
        let node = Arc::new(context.create_node("observer").build().await.unwrap());
        let retained = QosProfile {
            durability: QosDurability::TransientLocal,
            history: QosHistory::from_depth(1),
            ..Default::default()
        };
        let fields = node
            .subscriber::<FieldDimensions>("field_dimensions")
            .qos(retained)
            .build()
            .await
            .unwrap();
        let readiness = node
            .subscriber::<FieldDimensions>("field_dimensions")
            .qos(retained)
            .build()
            .await
            .unwrap();
        let limits = node
            .subscriber::<types::joint_limits::JointLimits>("joint_limits")
            .qos(retained)
            .build()
            .await
            .unwrap();
        let players = node
            .subscriber::<hsl_network_messages::PlayerNumber>("player_number")
            .qos(retained)
            .build()
            .await
            .unwrap();
        let client = RemoteParameterClient::new(
            node.clone(),
            "/global_parameters_test/global_parameter_provider",
        )
        .unwrap();
        let (updates, receiver) = watch::channel(None);
        let mut tasks = JoinSet::new();
        tasks.spawn(global_parameter_provider::run_boxed(context.clone()));
        tasks.spawn(synchronize_field_dimensions(
            client.clone(),
            readiness,
            receiver,
            overrides.path().to_string_lossy().into_owned(),
        ));
        tokio::time::timeout(Duration::from_secs(5), async {
            let mut dimensions = fields.recv().await.unwrap();
            assert_eq!(dimensions.length, 9.0);
            assert_eq!(dimensions.width, 6.0);
            limits.recv().await.unwrap().validate().unwrap();
            assert_eq!(
                players.recv().await.unwrap(),
                hsl_network_messages::PlayerNumber::Three
            );
            dimensions.width = 7.0;
            updates.send_replace(Some(dimensions));
            assert_eq!(fields.recv().await.unwrap().width, 7.0);
            let snapshot = client.get_snapshot().await.unwrap();
            assert!(snapshot.success);
            let value: serde_json::Value = serde_json::from_str(&snapshot.value_json).unwrap();
            assert_eq!(value["field_dimensions"]["width"], 7.0);
            let late = node
                .subscriber::<FieldDimensions>("field_dimensions")
                .qos(retained)
                .build()
                .await
                .unwrap();
            assert_eq!(late.recv().await.unwrap().width, 7.0);
        })
        .await
        .unwrap();
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        context.shutdown().unwrap();
    }

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
                assert!(
                    !motion.is_ready(),
                    "simulator must not publish behavior commands"
                );
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
        exercise_behavior_and_motion(&runtime, &mut io, &clock);
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

    fn exercise_behavior_and_motion(
        runtime: &tokio::runtime::Runtime,
        io: &mut Robotics,
        clock: &Clock,
    ) {
        use kinematics::joints::{Joints, body::LowerBodyJoints};
        use motion_inference::{
            inference::{InferenceCommand, InferenceResponse, PolicyExecution},
            node::{
                GETUP_INFERENCE_SERVICE, GetUpInferenceService, KICK_INFERENCE_SERVICE,
                KickInferenceService, WALK_INFERENCE_SERVICE, WalkInferenceService,
            },
        };
        use types::{filtered_game_state::FilteredGameState, motor_command::MotorCommand};

        let (mut tasks, _inference_node, _inference_status, mut requests, blackboard) = runtime
            .block_on(async {
                let node = io
                    .context
                    .create_node("inference_stub")
                    .build()
                    .await
                    .unwrap();
                let blackboard = node
                    .subscriber::<behavior_node::node::Blackboard>("behavior/blackboard")
                    .cache(1)
                    .build()
                    .await
                    .unwrap();
                let status = node
                    .publisher::<motion_inference::node::Status>(
                        motion_inference::node::STATUS_TOPIC,
                    )
                    .qos(QosProfile {
                        durability: QosDurability::TransientLocal,
                        ..Default::default()
                    })
                    .build()
                    .await
                    .unwrap();
                status
                    .publish(&motion_inference::node::Status {
                        time: clock.now(),
                        state: motion_inference::node::State::Initialized,
                    })
                    .await
                    .unwrap();
                let (sender, requests) = tokio::sync::mpsc::unbounded_channel();
                let mut tasks = JoinSet::new();
                macro_rules! stub_inference {
                    ($service:ty, $topic:expr, $variant:ident, $joints:ident) => {{
                        let mut service = node
                            .service_server::<$service>($topic)
                            .build()
                            .await
                            .unwrap();
                        let sender = sender.clone();
                        tasks.spawn(async move {
                            loop {
                                let (request, reply) =
                                    service.take_request_async().await?.into_parts();
                                let command = InferenceCommand::$variant(request.command);
                                sender.send(command).unwrap();
                                let result = Ok(InferenceResponse {
                                    joints: Box::new($joints::fill(MotorCommand {
                                        kp: 40.0,
                                        kd: 1.0,
                                        ..MotorCommand::zeros()
                                    })),
                                    execution: PolicyExecution {
                                        policy: command.policy(),
                                        started_at: request.requested_at,
                                        sensor_time: request.requested_at,
                                        progress: None,
                                    },
                                });
                                reply.reply_async(&result).await?;
                            }
                            #[allow(unreachable_code)]
                            Ok::<(), color_eyre::Report>(())
                        });
                    }};
                }
                stub_inference!(
                    WalkInferenceService,
                    WALK_INFERENCE_SERVICE,
                    Walk,
                    LowerBodyJoints
                );
                stub_inference!(
                    KickInferenceService,
                    KICK_INFERENCE_SERVICE,
                    Kick,
                    LowerBodyJoints
                );
                stub_inference!(
                    GetUpInferenceService,
                    GETUP_INFERENCE_SERVICE,
                    GetUp,
                    Joints
                );
                tasks.spawn(crate::simulated_sdk::run(io.context.clone()));
                tasks.spawn(global_parameter_provider::run_boxed(io.context.clone()));
                tasks.spawn(behavior_node::node::run_boxed(io.context.clone()));
                tasks.spawn(fall_detection::run_boxed(io.context.clone()));
                tasks.spawn(ball_state_composer::run_boxed(io.context.clone()));
                tasks.spawn(rule_obstacle_composer::run_boxed(io.context.clone()));
                tasks.spawn(hardware_interface::run_boxed(io.context.clone()));
                tasks.spawn(motion::run_boxed(io.context.clone()));
                tasks.spawn(head_motion::node::run_boxed(io.context.clone()));
                (tasks, node, status, requests, blackboard)
            });
        let mut step = |io: &Robotics| {
            io.publish_inputs().unwrap();
            let time = clock.now() + Duration::from_millis(20);
            io.publish_observation(
                Observation {
                    low_state: LowState {
                        motor_state_serial: vec![MotorState::default(); 22],
                        ..Default::default()
                    },
                    camera_matrix: CameraMatrix::default(),
                    ground_to_robot: Isometry3::identity(),
                },
                time,
            )
            .unwrap();
            io.publish_world(
                nalgebra::Isometry3::identity(),
                Some(([1.5, 0.0, 0.05], [0.2, 0.0, 0.0])),
                vec![[2.0, 1.0, 0.0]],
                time,
            )
            .unwrap();
            runtime.block_on(async { tokio::time::sleep(Duration::from_millis(10)).await });
            if let Some(result) = tasks.try_join_next() {
                panic!("stack exited: {result:?}");
            }
        };
        io.input_game.game_state = FilteredGameState::Initial;
        io.injection_enabled = true;
        io.input_motion = MotionCommand::Damping;
        for _ in 0..30 {
            step(io);
        }
        io.input_motion = MotionCommand::Prepare;
        for _ in 0..10 {
            step(io);
        }
        io.input_motion = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: linear_algebra::vector![0.25, -0.1],
            angular_velocity: 0.4,
        };
        for _ in 0..30 {
            step(io);
        }
        assert_eq!(io.active_motion(), io.input_motion);
        let mut saw_walk = false;
        while let Ok(command) = requests.try_recv() {
            if let InferenceCommand::Walk(walk) = command {
                saw_walk |= walk.velocity == linear_algebra::vector![0.25, -0.1]
                    && walk.angular_velocity == 0.4;
            }
        }
        assert!(saw_walk, "injected velocity never reached inference");
        assert!(
            io.latest_command()
                .unwrap()
                .motor_commands
                .iter()
                .any(|motor| motor.kp > 0.0)
        );
        io.clear_injection().unwrap();
        for _ in 0..10 {
            step(io);
        }
        assert!(matches!(io.active_motion(), MotionCommand::Stand { .. }));
        let board = blackboard.get_latest().unwrap();
        assert!(!board.is_injected_motion_command);
        assert_eq!(
            board.world_state.robot.primary_state,
            types::primary_state::PrimaryState::Initial
        );
        let ball = board.world_state.ball.unwrap();
        assert!((ball.ball_in_ground.x() - 1.5).abs() < 1e-5);
        assert!((ball.ball_in_ground_velocity.x() - 0.2).abs() < 1e-5);
        assert_eq!(board.world_state.obstacles.len(), 1);
        assert!(
            board
                .world_state
                .fall_detection
                .unwrap()
                .is_upright(clock.now())
        );
        io.input_game.game_state = FilteredGameState::Playing {
            ball_is_free: true,
            kick_off: false,
        };
        for _ in 0..40 {
            step(io);
        }
        assert!(
            matches!(
                io.active_motion(),
                MotionCommand::WalkWithVelocity { .. }
                    | MotionCommand::Walk { .. }
                    | MotionCommand::Kick { .. }
            ),
            "behavior did not pursue the ball: {:?}",
            io.active_motion()
        );
        runtime.block_on(async {
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
        });
    }
}
