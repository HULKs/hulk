use super::*;
use color_eyre::eyre::eyre;
use kinematics::joints::Joints;
use ros_z::{
    qos::{QosDurability, QosHistory, QosReliability},
    time::Time,
};
use types::{
    hardware_status::{ControlMode, HARDWARE_STATUS_TOPIC, HardwareStatus},
    joint_limits::JointLimits,
    motor_command::MotorCommand,
};

#[derive(Clone, Copy)]
struct ModeRequest {
    mode: ControlMode,
    sequence: u64,
}

struct ModeWorker {
    sequence: u64,
    commands: watch::Sender<Option<RetryCommand<ModeRequest>>>,
    acknowledged: watch::Receiver<Option<ModeRequest>>,
    task: tokio::task::JoinHandle<()>,
}

impl ModeWorker {
    fn new(client: Arc<loco_client::LocoClient>, diagnostics: Arc<RpcDiagnostics>) -> Self {
        let (commands, receiver) = watch::channel(None::<RetryCommand<ModeRequest>>);
        let (acknowledgements, acknowledged) = watch::channel(None);
        let task = tokio::spawn(run_retrying_rpc_worker(receiver, move |command| {
            let client = client.clone();
            let diagnostics = diagnostics.clone();
            let acknowledgements = acknowledgements.clone();
            async move {
                let attempt = diagnostics.begin(RpcActionKind::ChangeMode);
                retryable_rpc_call(
                    client.change_mode(sdk_mode(command.target.mode), command.timeout),
                    format!("request mode {:?}", command.target.mode),
                    attempt,
                )
                .await?;
                acknowledgements.send(Some(command.target))?;
                Ok(())
            }
        }));
        Self {
            sequence: 0,
            commands,
            acknowledged,
            task,
        }
    }
    fn request(&mut self, mode: ControlMode, timeout: Duration) -> Result<()> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| eyre!("mode generation exhausted"))?;
        self.commands.send(Some(RetryCommand {
            target: ModeRequest {
                mode,
                sequence: self.sequence,
            },
            timeout,
        }))?;
        Ok(())
    }
    fn acknowledged(&self) -> Option<ControlMode> {
        self.acknowledged
            .borrow()
            .as_ref()
            .filter(|ack| ack.sequence == self.sequence)
            .map(|ack| ack.mode)
    }
}
impl Drop for ModeWorker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn sdk_mode(mode: ControlMode) -> RobotMode {
    match mode {
        ControlMode::Damping => RobotMode::Damping,
        ControlMode::Prepare => RobotMode::Prepare,
        ControlMode::Custom => RobotMode::Custom,
    }
}

struct Actuator {
    command: Option<(Time, Time, RobotCommand)>,
    limits: Option<JointLimits>,
    desired: ControlMode,
    mode_since: Time,
    initialized: bool,
    fault: Option<String>,
    saw_damping: bool,
}
impl Actuator {
    fn new(now: Time) -> Self {
        Self {
            command: None,
            limits: None,
            desired: ControlMode::Damping,
            mode_since: now,
            initialized: false,
            fault: None,
            saw_damping: false,
        }
    }
    fn fail(&mut self, reason: impl Into<String>) {
        if self.fault.is_none() {
            self.fault = Some(reason.into());
            self.saw_damping = false;
        }
        self.command = None;
    }
    fn receive(&mut self, command: RobotCommand, source: Time, now: Time, p: &Parameters) {
        if source > now || now.duration_since(source) > p.command_timeout {
            self.fail("expired robot command");
            return;
        }
        if self
            .command
            .as_ref()
            .is_some_and(|(old, _, _)| source <= *old)
        {
            return;
        }
        if self.fault.is_some() {
            match command {
                RobotCommand::Damping => self.saw_damping = true,
                RobotCommand::Prepare if self.saw_damping => {
                    self.fault = None;
                    self.saw_damping = false;
                }
                _ => return,
            }
        }
        self.command = Some((source, now, command));
    }
    fn output(&mut self, now: Time, p: &Parameters) -> (ControlMode, LowCommand) {
        if self.command.as_ref().is_some_and(|(source, receipt, _)| {
            *source > now
                || *receipt > now
                || now.duration_since(*source) > p.command_timeout
                || now.duration_since(*receipt) > p.command_timeout
        }) {
            self.fail("robot command watchdog expired");
        }
        if self.fault.is_some() || !self.initialized {
            return (ControlMode::Damping, protective_command());
        }
        let Some((_, _, command)) = &self.command else {
            return (ControlMode::Damping, protective_command());
        };
        match command {
            RobotCommand::Damping => (ControlMode::Damping, protective_command()),
            RobotCommand::Prepare => (ControlMode::Prepare, protective_command()),
            RobotCommand::EnableCustom => (ControlMode::Custom, protective_command()),
            RobotCommand::Custom { joints_command } => {
                let Some(limits) = &self.limits else {
                    self.fail("joint limits unavailable");
                    return (ControlMode::Damping, protective_command());
                };
                // The transport is an unchecked boundary, even if Motion validated its output.
                match (RobotCommand::Custom {
                    joints_command: joints_command.clone(),
                })
                .clamp(limits)
                {
                    Ok(RobotCommand::Custom { joints_command }) => (
                        ControlMode::Custom,
                        low_command_from_joints_command(joints_command),
                    ),
                    _ => {
                        self.fail("invalid actuator command");
                        (ControlMode::Damping, protective_command())
                    }
                }
            }
        }
    }
    async fn tick(
        &mut self,
        node: &Node,
        p: &Parameters,
        modes: &mut ModeWorker,
        publisher: &JointControlPublisher,
        statuses: &Publisher<HardwareStatus>,
        sent_commands: &Publisher<LowCommand>,
    ) -> Result<()> {
        let now = node.clock().now();
        let acknowledged = modes.acknowledged();
        if acknowledged == Some(ControlMode::Damping) {
            self.initialized = true;
        }
        if modes.task.is_finished() {
            return Err(eyre!("mode worker stopped"));
        }
        if acknowledged != Some(self.desired)
            && now.duration_since(self.mode_since) > p.mode_transition_timeout
        {
            self.fail("hardware mode acknowledgement timed out");
        }
        let (desired, command) = self.output(now, p);
        if desired != self.desired {
            modes.request(desired, p.sdk_request_timeout)?;
            self.desired = desired;
            self.mode_since = now;
        }
        let acknowledged = modes.acknowledged();
        // Protective targets are streamed immediately, including while an older mode RPC is in flight.
        let command = if desired == ControlMode::Custom
            && acknowledged == Some(ControlMode::Custom)
            && self.fault.is_none()
        {
            command
        } else {
            protective_command()
        };
        publisher.publish(&command).await?;
        sent_commands.publish(&command).await?;
        statuses
            .publish(&HardwareStatus {
                time: now,
                desired,
                acknowledged,
                command_time: self.command.as_ref().map(|c| c.0),
                fault: self.fault.clone(),
            })
            .await?;
        Ok(())
    }
}

pub(super) async fn run(
    ctx: Arc<Context>,
    node: Arc<Node>,
    parameters: Arc<NodeParameters<Parameters>>,
    diagnostics: Arc<RpcDiagnostics>,
) -> Result<()> {
    let qos = QosProfile {
        history: QosHistory::from_depth(1),
        reliability: QosReliability::BestEffort,
        ..Default::default()
    };
    let commands = node
        .subscriber::<RobotCommand>(ROBOT_COMMAND_TOPIC)
        .qos(qos)
        .build()
        .await?;
    let limits = node
        .subscriber::<JointLimits>("joint_limits")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..qos
        })
        .build()
        .await?;
    let statuses = node
        .publisher::<HardwareStatus>(HARDWARE_STATUS_TOPIC)
        .qos(qos)
        .build()
        .await?;
    let sent_commands = node
        .publisher::<LowCommand>("hardware_interface/joint_command")
        .qos(qos)
        .build()
        .await?;
    let publisher = JointControlPublisher::new(ctx.session()).await?;
    let client = Arc::new(loco_client::LocoClient::new(ctx.session()).await?);
    let mut modes = ModeWorker::new(client, diagnostics);
    let p = parameters.snapshot();
    modes.request(ControlMode::Damping, p.typed().sdk_request_timeout)?;
    let mut actuator = Actuator::new(node.clock().now());
    let mut timer = node.create_timer(p.typed().joint_control_message_interval);
    loop {
        tokio::select! {
            received=commands.recv_with_metadata()=> {
                let r=received?;let time=r.source_time;
                actuator.receive(r.into_message(),time,node.clock().now(),parameters.snapshot().typed());
            }
            received=limits.recv()=> {
                let limits=received?;
                match limits.validate() {Ok(())=>actuator.limits=Some(limits),Err(reason)=>{actuator.limits=None;actuator.fail(reason);}}
            }
            _=timer.tick()=>actuator.tick(&node,parameters.snapshot().typed(),&mut modes,&publisher,&statuses,&sent_commands).await?,
        }
    }
}

fn protective_command() -> LowCommand {
    low_command_from_joints_command(Joints::fill(MotorCommand::damping()))
}

pub(super) async fn protect(session: &zenoh::Session, timeout: Duration) -> Result<()> {
    let publisher = JointControlPublisher::new(session).await?;
    publisher.publish(&protective_command()).await?;
    loco_client::LocoClient::new(session)
        .await?
        .change_mode(RobotMode::Damping, timeout)
        .await
}
