use super::*;
use crate::inputs::{Latest, Sample};
use booster::{JointsMotorState, LowState};
use ros_z::{
    prelude::*,
    qos::{QosHistory, QosReliability},
    time::Time,
};
use std::time::Instant;
use tracing::{error, warn};
use types::hardware_status::{ControlMode, HARDWARE_STATUS_TOPIC, HardwareStatus};

struct Inputs {
    commands: Latest<MotionCommand>,
    sensors: Latest<LowState>,
    limits: Latest<JointLimits>,
    hardware: Latest<HardwareStatus>,
    inference: Latest<motion_inference::node::Status>,
}
struct Frame {
    time: Time,
    command: Arc<Sample<MotionCommand>>,
    position: Joints<f32>,
    limits: Arc<Sample<JointLimits>>,
    hardware: Arc<Sample<HardwareStatus>>,
}
impl Inputs {
    async fn new(node: &Node, qos: QosProfile) -> Result<Self> {
        let retained = QosProfile {
            durability: QosDurability::TransientLocal,
            ..qos
        };
        Ok(Self {
            commands: Latest::subscribe(node, "behavior/motion_command", qos).await?,
            sensors: Latest::subscribe(node, "inputs/low_state", qos).await?,
            limits: Latest::subscribe(node, "joint_limits", retained).await?,
            hardware: Latest::subscribe(node, HARDWARE_STATUS_TOPIC, qos).await?,
            inference: Latest::subscribe(node, motion_inference::node::STATUS_TOPIC, retained)
                .await?,
        })
    }
    fn frame(&self, clock: &Clock, p: &Parameters) -> Result<Frame> {
        let command = self.commands.snapshot().wrap_err("behavior command")?;
        let sensor = self.sensors.snapshot().wrap_err("body sensors")?;
        let hardware = self.hardware.snapshot().wrap_err("hardware state")?;
        let limits = self
            .limits
            .latest()
            .ok_or_else(|| eyre!("joint limits unavailable"))?;
        // Validate one set of immutable snapshots against a clock read after all of them.
        let now = clock.now();
        command
            .validate_freshness(now, p.maximum_command_age)
            .wrap_err("behavior command")?;
        sensor
            .validate_freshness(now, p.maximum_sensor_age)
            .wrap_err("body sensors")?;
        hardware
            .validate_freshness(now, p.maximum_hardware_age)
            .wrap_err("hardware state")?;
        limits.received.validate().map_err(|e| eyre!(e))?;
        Ok(Frame {
            time: now,
            position: body_position(&sensor.received)?,
            command,
            limits,
            hardware,
        })
    }
}

fn body_position(sensor: &LowState) -> Result<Joints<f32>> {
    let motors = sensor.serial_motor_states()?;
    let position = motors.positions();
    ensure!(
        position
            .into_iter()
            .chain(motors.velocities())
            .chain(sensor.imu_state.roll_pitch_yaw.inner.iter().copied())
            .chain(sensor.imu_state.angular_velocity.inner.iter().copied())
            .all(f32::is_finite),
        "invalid body sensors"
    );
    Ok(position)
}

pub(super) async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("motion").build().await?;
    let parameters = node.bind_parameter_as::<Parameters>("motion")?;
    parameters.add_validation_hook(Parameters::validate)?;
    let qos = QosProfile {
        reliability: QosReliability::BestEffort,
        history: QosHistory::from_depth(1),
        ..Default::default()
    };
    let inputs = Inputs::new(&node, qos).await?;
    let outputs = node
        .publisher::<RobotCommand>(ROBOT_COMMAND_TOPIC)
        .qos(qos)
        .build()
        .await?;
    let mut motion = MotionState {
        walk_inference_client: node
            .service_client::<WalkInferenceService>(WALK_INFERENCE_SERVICE)
            .qos(qos)
            .build()
            .await?,
        kick_inference_client: node
            .service_client::<KickInferenceService>(KICK_INFERENCE_SERVICE)
            .qos(qos)
            .build()
            .await?,
        get_up_inference_client: node
            .service_client::<GetUpInferenceService>(GETUP_INFERENCE_SERVICE)
            .qos(qos)
            .build()
            .await?,
        head_motion_client: node
            .service_client::<HeadMotionService>(HEAD_MOTION_SERVICE_TOPIC)
            .build()
            .await?,
        generation: node.clock().now().as_nanos() as u64,
        active: false,
        last_policy: None,
        last_arms: TimeWrapper {
            time: node.clock().now(),
            inner: UpperBodyJoints::fill(0.0),
        },
    };
    let mut safety = ControlSafety::default();
    let mut timer = node.create_timer(Duration::from_millis(20));
    loop {
        timer.tick().await;
        let p = parameters.snapshot();
        let command = cycle(&node, &inputs, &mut motion, &mut safety, p.typed()).await;
        outputs.publish(&command).await?;
    }
}

#[derive(Default)]
struct ControlSafety {
    fault: Option<String>,
    saw_damping: bool,
    has_actuated: bool,
    last_input_warning: Option<Instant>,
}
impl ControlSafety {
    fn fail(&mut self, error: impl std::fmt::Display) {
        if self.fault.is_none() {
            error!("motion safety fault: {error:#}");
            self.fault = Some(format!("{error:#}"));
            self.saw_damping = false;
        }
    }
    fn reject_input(&mut self, error: color_eyre::Report) {
        if self.has_actuated {
            self.fail(error);
        } else if self
            .last_input_warning
            .is_none_or(|time| time.elapsed() >= Duration::from_secs(1))
        {
            warn!("motion input rejected before actuation; commanding Damping: {error:#}");
            self.last_input_warning = Some(Instant::now());
        }
    }
    fn rearm(&mut self, command: &MotionCommand) -> bool {
        match command {
            MotionCommand::Damping => {
                self.saw_damping = true;
                false
            }
            MotionCommand::Prepare if self.saw_damping => {
                self.fault = None;
                self.saw_damping = false;
                self.has_actuated = false;
                true
            }
            _ => false,
        }
    }
}

async fn cycle(
    node: &Node,
    inputs: &Inputs,
    motion: &mut MotionState,
    safety: &mut ControlSafety,
    p: &Parameters,
) -> RobotCommand {
    let request = inputs.commands.fresh(node.clock(), p.maximum_command_age);
    if let Ok(request) = &request
        && matches!(request.received.message, MotionCommand::Damping)
    {
        if safety.has_actuated
            && let Err(error) = inputs.frame(node.clock(), p)
        {
            safety.fail(error);
        }
        if safety.fault.is_some() {
            safety.rearm(&request.received);
        }
        motion.deactivate();
        return RobotCommand::Damping;
    }
    let frame = match inputs.frame(node.clock(), p) {
        Ok(frame) => frame,
        Err(error) => {
            safety.reject_input(error);
            motion.deactivate();
            return RobotCommand::Damping;
        }
    };
    if safety.fault.is_some() && !safety.rearm(&frame.command.received) {
        motion.deactivate();
        return RobotCommand::Damping;
    }
    if matches!(frame.command.received.message, MotionCommand::Prepare) {
        motion.deactivate();
        return RobotCommand::Prepare;
    }
    if let Some(fault) = &frame.hardware.received.fault {
        safety.fail(fault);
        motion.deactivate();
        return RobotCommand::Damping;
    }
    let initialized = inputs
        .inference
        .latest()
        .is_some_and(|s| !matches!(s.received.state, motion_inference::node::State::Idle));
    if !initialized {
        motion.deactivate();
        return RobotCommand::Damping;
    }
    if frame.hardware.received.acknowledged != Some(ControlMode::Custom)
        || frame.hardware.received.desired != ControlMode::Custom
    {
        if motion.active {
            safety.fail("lost Custom mode acknowledgement");
            motion.deactivate();
            return RobotCommand::Damping;
        }
        return RobotCommand::EnableCustom;
    }
    if !motion.active {
        motion.last_arms = TimeWrapper {
            time: frame.time,
            inner: frame.position.upper_body_as_ref().map(|v| *v),
        };
    }
    match infer_and_validate(node, inputs, motion, &frame, p).await {
        Ok(command) => {
            safety.has_actuated |= matches!(command, RobotCommand::Custom { .. });
            command
        }
        Err(error) => {
            safety.fail(error);
            motion.deactivate();
            RobotCommand::Damping
        }
    }
}

async fn infer_and_validate(
    node: &Node,
    inputs: &Inputs,
    motion: &mut MotionState,
    frame: &Frame,
    p: &Parameters,
) -> Result<RobotCommand> {
    let plan = MotionPlan::from_motion_command(&frame.command.received, &p.walking)?;
    let command = motion
        .infer(plan, node.clock(), p, &frame.limits.received)
        .await?;
    let fresh = inputs.frame(node.clock(), p)?;
    if matches!(
        fresh.command.received.message,
        MotionCommand::Damping | MotionCommand::Prepare
    ) {
        motion.deactivate();
        return Ok(
            if matches!(fresh.command.received.message, MotionCommand::Prepare) {
                RobotCommand::Prepare
            } else {
                RobotCommand::Damping
            },
        );
    }
    ensure!(
        fresh.hardware.received.fault.is_none()
            && fresh.hardware.received.acknowledged == Some(ControlMode::Custom)
            && fresh.hardware.received.desired == ControlMode::Custom,
        "hardware no longer authorizes Custom"
    );
    command.clamp(&fresh.limits.received)
}
