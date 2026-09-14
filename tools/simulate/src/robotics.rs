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
    filtered_game_controller_state::FilteredGameControllerState, motion_command::MotionCommand,
    time_wrapper::TimeWrapper,
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
            "Head-yaw sine dummy active; motion node disabled".to_owned()
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
            tasks.spawn(head_motion::node::run_boxed(ctx.clone()));
            tasks.spawn(motion_inference::run_boxed(ctx.clone()));
            tasks.spawn(booster_sdk_interface::run_boxed(ctx));
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
        // Exercise the real hardware interface as well: the dummy's ROS-Z joint
        // command must become a raw CDR LowCommand even with unanswered mode RPCs.
        runtime.block_on(async {
            let hardware = tokio::spawn(booster_sdk_interface::run_boxed(io.context.clone()));
            let dummy = tokio::spawn(crate::motion_dummy::run(io.context.clone()));
            let mut received = false;
            for _ in 0..20 {
                clock.advance(Duration::from_millis(20)).unwrap();
                if tokio::time::timeout(Duration::from_millis(100), io.commands.changed())
                    .await
                    .is_ok()
                {
                    received = true;
                    break;
                }
            }
            hardware.abort();
            dummy.abort();
            let _ = hardware.await;
            let _ = dummy.await;
            assert!(
                received,
                "dummy command never reached the raw hardware topic"
            );
            let command = io.latest_command().unwrap();
            assert_eq!(command.motor_commands.len(), 22);
            assert!(
                command
                    .motor_commands
                    .iter()
                    .enumerate()
                    .all(|(i, motor)| (i == 0 || motor.position == 0.0)
                        && motor.kp > 0.0
                        && motor.kd > 0.0)
            );
            assert!(command.motor_commands[0].position.abs() <= 0.5);
            assert_eq!(command.motor_commands[0].kp, 10.0);
            assert_eq!(command.motor_commands[10].kp, 80.0);
        });
        drop(io);
        server.shutdown().unwrap();
    }
}
