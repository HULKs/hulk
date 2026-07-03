use std::{
    borrow::Cow,
    boxed::Box,
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use booster::{LedColor, RobotMode};
use color_eyre::{Result, eyre::WrapErr};
use kinematics::joints::head::HeadJoints;
use retry_worker::{RetryCommand, run_retrying_rpc_worker};
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tracing::{error, info};
use types::motion_command::MotionCommand;

mod control;
mod kick_transport;
mod light_client;
mod loco_client;
mod retry_worker;
mod rpc_transport;

pub use light_client::LightClient;
pub use loco_client::LocoClient;
pub use rpc_transport::ZenohRpcClient;

const MOTION_COMMAND_TOPIC: &str = "behavior/motion_command";

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct WalkingParameters {
    pub hybrid_align_distance: f32,
    pub max_alignment_rate: f32,
    pub deceleration_distance: f32,
}

#[derive(Debug, Serialize, Deserialize, Message)]
pub enum LedCommand {
    SetParam { r: u8, g: u8, b: u8 },
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub walking: WalkingParameters,
    pub move_robot_message_interval: std::time::Duration,
    pub kicking: types::parameters::BoosterKickingParameters,
    pub rotate_head_message_interval: std::time::Duration,
    pub sdk_request_timeout: std::time::Duration,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MotionCommandKind {
    Damping,
    Prepare,
    Stand,
    StandUp,
    VisualKick,
    Walk,
}

impl MotionCommandKind {
    fn from_command(command: &MotionCommand) -> Self {
        match command {
            MotionCommand::Damping => Self::Damping,
            MotionCommand::Prepare => Self::Prepare,
            MotionCommand::Stand { .. } => Self::Stand,
            MotionCommand::StandUp => Self::StandUp,
            MotionCommand::VisualKick { .. } => Self::VisualKick,
            MotionCommand::Walk { .. } | MotionCommand::WalkWithVelocity { .. } => Self::Walk,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum RpcActionKind {
    ChangeMode,
    GetUp,
    LedControl,
    MoveRobot,
    RotateHead,
    KickPublish,
    VisualKick,
}

impl RpcActionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::ChangeMode => "change_mode",
            Self::GetUp => "get_up",
            Self::LedControl => "led_control",
            Self::MoveRobot => "move_robot",
            Self::RotateHead => "rotate_head",
            Self::KickPublish => "kick_publish",
            Self::VisualKick => "visual_kick",
        }
    }
}

#[derive(Default)]
struct RpcDiagnostics {
    next_sequence: AtomicU64,
    change_mode_in_flight: AtomicUsize,
    get_up_in_flight: AtomicUsize,
    led_control_in_flight: AtomicUsize,
    move_robot_in_flight: AtomicUsize,
    rotate_head_in_flight: AtomicUsize,
    kick_publish_in_flight: AtomicUsize,
    visual_kick_in_flight: AtomicUsize,
}

impl RpcDiagnostics {
    fn begin(self: &Arc<Self>, kind: RpcActionKind) -> RpcAttempt {
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let counter = self.counter(kind);
        let in_flight = counter.fetch_add(1, Ordering::Relaxed) + 1;
        RpcAttempt {
            diagnostics: self.clone(),
            kind,
            sequence,
            started_at: Instant::now(),
            in_flight_at_start: in_flight,
        }
    }

    fn counter(&self, kind: RpcActionKind) -> &AtomicUsize {
        match kind {
            RpcActionKind::ChangeMode => &self.change_mode_in_flight,
            RpcActionKind::GetUp => &self.get_up_in_flight,
            RpcActionKind::LedControl => &self.led_control_in_flight,
            RpcActionKind::MoveRobot => &self.move_robot_in_flight,
            RpcActionKind::RotateHead => &self.rotate_head_in_flight,
            RpcActionKind::KickPublish => &self.kick_publish_in_flight,
            RpcActionKind::VisualKick => &self.visual_kick_in_flight,
        }
    }
}

struct RpcAttempt {
    diagnostics: Arc<RpcDiagnostics>,
    kind: RpcActionKind,
    sequence: u64,
    started_at: Instant,
    in_flight_at_start: usize,
}

impl RpcAttempt {
    fn finish(self, status: &'static str) {
        let remaining_in_flight = self
            .diagnostics
            .counter(self.kind)
            .fetch_sub(1, Ordering::Relaxed)
            .saturating_sub(1);
        let elapsed_ms = self.started_at.elapsed().as_secs_f64() * 1000.0;
        info!(
            target: "booster_interface::rpc",
            sequence = self.sequence,
            action = self.kind.as_str(),
            status,
            elapsed_ms,
            in_flight_at_start = self.in_flight_at_start,
            remaining_in_flight,
            "booster rpc completed"
        );
    }
}

struct InterfaceState {
    assumed_mode: control::DesiredMode,
    last_motion_kind: MotionCommandKind,
    visual_kick_active: bool,
    active_get_up_request: Option<GetUpRequest>,
    next_get_up_request: u64,
    last_move_robot: std::time::Instant,
    last_rotate_head: std::time::Instant,
    last_kick: std::time::Instant,
    last_logged_motion_kind: Option<MotionCommandKind>,
    last_logged_desired_mode: Option<control::DesiredMode>,
    last_logged_assumed_mode: Option<control::DesiredMode>,
    last_logged_visual_kick_active: Option<bool>,
    last_logged_head_present: Option<bool>,
}

impl InterfaceState {
    fn new(now: std::time::Instant) -> Self {
        Self {
            assumed_mode: control::DesiredMode::Damping,
            last_motion_kind: MotionCommandKind::Damping,
            visual_kick_active: false,
            active_get_up_request: None,
            next_get_up_request: 0,
            last_move_robot: now,
            last_rotate_head: now,
            last_kick: now,
            last_logged_motion_kind: None,
            last_logged_desired_mode: None,
            last_logged_assumed_mode: None,
            last_logged_visual_kick_active: None,
            last_logged_head_present: None,
        }
    }

    fn start_get_up_request(&mut self) -> GetUpRequest {
        self.next_get_up_request += 1;
        let request = GetUpRequest(self.next_get_up_request);
        self.active_get_up_request = Some(request);
        request
    }

    fn clear_get_up_request(&mut self) -> bool {
        self.active_get_up_request.take().is_some()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GetUpRequest(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DesiredLed {
    Set(LedColor),
    Stop,
}

fn due(last: std::time::Instant, now: std::time::Instant, interval: Duration) -> bool {
    now.duration_since(last) >= interval
}

fn should_send_move(command: &MotionCommand) -> bool {
    matches!(
        command,
        MotionCommand::Stand { .. }
            | MotionCommand::Walk { .. }
            | MotionCommand::WalkWithVelocity { .. }
    )
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("booster_interface")
        .build()
        .await
        .wrap_err("failed to create booster_interface node")?;
    let parameters = node
        .bind_parameter_as::<Parameters>("booster_interface")
        .wrap_err("failed to bind booster_interface parameters")?;
    let light_control_client = Arc::new(
        light_client::LightClient::new(ctx.session())
            .await
            .wrap_err("failed to create LightClient")?,
    );
    let loco_client = Arc::new(
        loco_client::LocoClient::new(ctx.session())
            .await
            .wrap_err("failed to create LocoClient")?,
    );
    let kick_ball_publisher = kick_transport::KickBallPublisher::new(ctx.session())
        .await
        .wrap_err("failed to create kick ball publisher")?;

    let motion_command_cache = node
        .subscriber::<MotionCommand>(MOTION_COMMAND_TOPIC)
        .cache(1)
        .build()
        .await
        .wrap_err("failed to build motion_command cache")?;
    let head_joints_cache = node
        .subscriber::<HeadJoints<f32>>("head_joints_command")
        .cache(1)
        .build()
        .await
        .wrap_err("failed to build head_joints_command cache")?;
    let led_command_sub = node
        .subscriber::<LedCommand>("commands/led_command")
        .build()
        .await
        .wrap_err("failed to build commands/led_command subscriber")?;
    let kick_ball_publisher = Arc::new(kick_ball_publisher);
    let rpc_diagnostics = Arc::new(RpcDiagnostics::default());
    let mode_command_sender = spawn_mode_worker(loco_client.clone(), rpc_diagnostics.clone());
    let visual_kick_command_sender =
        spawn_visual_kick_worker(loco_client.clone(), rpc_diagnostics.clone());
    let get_up_command_sender = spawn_get_up_worker(loco_client.clone(), rpc_diagnostics.clone());
    let led_command_sender =
        spawn_led_worker(light_control_client.clone(), rpc_diagnostics.clone());

    let mut state = InterfaceState::new(std::time::Instant::now());
    let mut tick = node.create_timer(std::time::Duration::from_millis(10));

    loop {
        tokio::select! {
            led_command = led_command_sub.recv() => {
                let led_command = led_command?;
                info!(target: "booster_interface::input", ?led_command, "received led command");
                let timeout = parameters.snapshot().typed().sdk_request_timeout;
                let desired_led = desired_led_for(led_command);
                send_retry_command(&led_command_sender, desired_led, timeout, "led_control");
            }
            _ = tick.tick() => {
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();
                let Some(motion_command) = motion_command_cache.get_latest() else {
                    continue;
                };
                let motion_command = motion_command.as_ref();
                let head_joints = head_joints_cache
                    .get_latest()
                    .map(|head_joints| *head_joints);
                let now = std::time::Instant::now();
                let timeout = parameters.sdk_request_timeout;
                let motion_kind = MotionCommandKind::from_command(motion_command);

                if state.visual_kick_active && motion_kind != MotionCommandKind::VisualKick {
                    if state.assumed_mode == control::DesiredMode::Soccer {
                        send_retry_command(&visual_kick_command_sender, false, timeout, "visual_kick");
                    }
                    state.visual_kick_active = false;
                }

                let desired_mode = control::desired_mode_for(motion_command);
                let head_present = head_joints.is_some();
                if state.last_logged_motion_kind != Some(motion_kind)
                    || state.last_logged_desired_mode != Some(desired_mode)
                    || state.last_logged_assumed_mode != Some(state.assumed_mode)
                    || state.last_logged_visual_kick_active != Some(state.visual_kick_active)
                    || state.last_logged_head_present != Some(head_present)
                {
                    info!(
                        target: "booster_interface::input",
                        ?motion_kind,
                        ?desired_mode,
                        assumed_mode = ?state.assumed_mode,
                        visual_kick_active = state.visual_kick_active,
                        head_present,
                        "booster input state changed"
                    );
                    state.last_logged_motion_kind = Some(motion_kind);
                    state.last_logged_desired_mode = Some(desired_mode);
                    state.last_logged_assumed_mode = Some(state.assumed_mode);
                    state.last_logged_visual_kick_active = Some(state.visual_kick_active);
                    state.last_logged_head_present = Some(head_present);
                }

                if desired_mode != state.assumed_mode {
                    let mode = robot_mode_for(desired_mode);
                    send_retry_command(&mode_command_sender, mode, timeout, "change_mode");
                    state.assumed_mode = desired_mode;
                }

                if motion_kind == MotionCommandKind::StandUp
                    && state.last_motion_kind != MotionCommandKind::StandUp
                    && state.assumed_mode == control::DesiredMode::Soccer
                {
                    let request = state.start_get_up_request();
                    send_retry_command(&get_up_command_sender, request, timeout, "get_up");
                } else if (motion_kind != MotionCommandKind::StandUp
                    || state.assumed_mode != control::DesiredMode::Soccer)
                    && state.clear_get_up_request()
                {
                    clear_retry_command(&get_up_command_sender, "get_up");
                }

                if motion_kind == MotionCommandKind::VisualKick
                    && state.assumed_mode == control::DesiredMode::Soccer
                {
                    let entering_visual_kick = !state.visual_kick_active;
                    if entering_visual_kick
                        || due(state.last_kick, now, parameters.kicking.kick_message_interval)
                    {
                        if let Some(kick) = control::kick_from_motion_command(
                            motion_command,
                            node.clock().now(),
                            &parameters.kicking,
                        ) {
                            let attempt = rpc_diagnostics.begin(RpcActionKind::KickPublish);
                            info!(
                                target: "booster_interface::rpc",
                                sequence = attempt.sequence,
                                action = "kick_publish",
                                in_flight = attempt.in_flight_at_start,
                                "booster rpc scheduled"
                            );
                            let kick_ball_publisher = kick_ball_publisher.clone();
                            tokio::spawn(async move {
                                let _ = await_rpc_call_with_timeout(
                                    kick_ball_publisher.publish(&kick),
                                    timeout,
                                    "publish visual kick command",
                                    attempt,
                                )
                                .await;
                            });
                        }
                        state.last_kick = now;
                    }

                    if entering_visual_kick {
                        send_retry_command(&visual_kick_command_sender, true, timeout, "visual_kick");
                        state.visual_kick_active = true;
                    }
                }

                if should_send_move(motion_command)
                    && state.assumed_mode == control::DesiredMode::Soccer
                    && due(state.last_move_robot, now, parameters.move_robot_message_interval)
                {
                    let step = control::step_from_motion_command(motion_command, &parameters.walking);
                    let attempt = rpc_diagnostics.begin(RpcActionKind::MoveRobot);
                    info!(
                        target: "booster_interface::rpc",
                        sequence = attempt.sequence,
                        action = "move_robot",
                        forward = step.forward,
                        left = step.left,
                        turn = step.turn,
                        in_flight = attempt.in_flight_at_start,
                        "booster rpc scheduled"
                    );
                    let loco_client = loco_client.clone();
                    tokio::spawn(async move {
                        let _ = await_rpc_call(
                            loco_client.move_robot(step.forward, step.left, step.turn, timeout),
                            "send move_robot",
                            attempt,
                        )
                        .await;
                    });
                    state.last_move_robot = now;
                }

                if let Some(head_joints) = head_joints
                    && state.assumed_mode == control::DesiredMode::Soccer
                    && due(
                        state.last_rotate_head,
                        now,
                        parameters.rotate_head_message_interval,
                    )
                {
                    let attempt = rpc_diagnostics.begin(RpcActionKind::RotateHead);
                    info!(
                        target: "booster_interface::rpc",
                        sequence = attempt.sequence,
                        action = "rotate_head",
                        pitch = head_joints.pitch,
                        yaw = head_joints.yaw,
                        in_flight = attempt.in_flight_at_start,
                        "booster rpc scheduled"
                    );
                    let loco_client = loco_client.clone();
                    tokio::spawn(async move {
                        let _ = await_rpc_call(
                            loco_client.rotate_head(head_joints.pitch, head_joints.yaw, timeout),
                            "rotate head",
                            attempt,
                        )
                        .await;
                    });
                    state.last_rotate_head = now;
                }

                state.last_motion_kind = motion_kind;
            }
        }
    }
}

fn desired_led_for(led_command: LedCommand) -> DesiredLed {
    match led_command {
        LedCommand::SetParam { r, g, b } => DesiredLed::Set(LedColor { r, g, b }),
        LedCommand::Stop => DesiredLed::Stop,
    }
}

fn robot_mode_for(desired_mode: control::DesiredMode) -> RobotMode {
    match desired_mode {
        control::DesiredMode::Damping => RobotMode::Damping,
        control::DesiredMode::Prepare => RobotMode::Prepare,
        control::DesiredMode::Soccer => RobotMode::Soccer,
    }
}

fn spawn_mode_worker(
    loco_client: Arc<loco_client::LocoClient>,
    rpc_diagnostics: Arc<RpcDiagnostics>,
) -> watch::Sender<Option<RetryCommand<RobotMode>>> {
    let (sender, receiver) = watch::channel(None::<RetryCommand<RobotMode>>);
    tokio::spawn(run_retrying_rpc_worker(receiver, move |command| {
        let loco_client = loco_client.clone();
        let rpc_diagnostics = rpc_diagnostics.clone();
        async move {
            let mode = command.target;
            let attempt = rpc_diagnostics.begin(RpcActionKind::ChangeMode);
            info!(
                target: "booster_interface::rpc",
                sequence = attempt.sequence,
                action = "change_mode",
                ?mode,
                in_flight = attempt.in_flight_at_start,
                "booster rpc scheduled"
            );
            retryable_rpc_call(
                loco_client.change_mode(mode, command.timeout),
                format!("request booster mode {mode:?}"),
                attempt,
            )
            .await
        }
    }));
    sender
}

fn spawn_visual_kick_worker(
    loco_client: Arc<loco_client::LocoClient>,
    rpc_diagnostics: Arc<RpcDiagnostics>,
) -> watch::Sender<Option<RetryCommand<bool>>> {
    let (sender, receiver) = watch::channel(None::<RetryCommand<bool>>);
    tokio::spawn(run_retrying_rpc_worker(receiver, move |command| {
        let loco_client = loco_client.clone();
        let rpc_diagnostics = rpc_diagnostics.clone();
        async move {
            let enabled = command.target;
            let attempt = rpc_diagnostics.begin(RpcActionKind::VisualKick);
            info!(
                target: "booster_interface::rpc",
                sequence = attempt.sequence,
                action = "visual_kick",
                enabled,
                in_flight = attempt.in_flight_at_start,
                "booster rpc scheduled"
            );
            let operation = if enabled {
                "start visual kick"
            } else {
                "stop visual kick"
            };
            retryable_rpc_call(
                loco_client.visual_kick(enabled, command.timeout),
                operation,
                attempt,
            )
            .await
        }
    }));
    sender
}

fn spawn_get_up_worker(
    loco_client: Arc<loco_client::LocoClient>,
    rpc_diagnostics: Arc<RpcDiagnostics>,
) -> watch::Sender<Option<RetryCommand<GetUpRequest>>> {
    let (sender, receiver) = watch::channel(None::<RetryCommand<GetUpRequest>>);
    tokio::spawn(run_retrying_rpc_worker(receiver, move |command| {
        let loco_client = loco_client.clone();
        let rpc_diagnostics = rpc_diagnostics.clone();
        async move {
            let request = command.target;
            let attempt = rpc_diagnostics.begin(RpcActionKind::GetUp);
            info!(
                target: "booster_interface::rpc",
                sequence = attempt.sequence,
                action = "get_up",
                request = request.0,
                in_flight = attempt.in_flight_at_start,
                "booster rpc scheduled"
            );
            retryable_rpc_call(
                loco_client.get_up(command.timeout),
                "request get_up",
                attempt,
            )
            .await
        }
    }));
    sender
}

fn spawn_led_worker(
    light_control_client: Arc<light_client::LightClient>,
    rpc_diagnostics: Arc<RpcDiagnostics>,
) -> watch::Sender<Option<RetryCommand<DesiredLed>>> {
    let (sender, receiver) = watch::channel(None::<RetryCommand<DesiredLed>>);
    tokio::spawn(run_retrying_rpc_worker(receiver, move |command| {
        let light_control_client = light_control_client.clone();
        let rpc_diagnostics = rpc_diagnostics.clone();
        async move {
            let desired_led = command.target;
            let attempt = rpc_diagnostics.begin(RpcActionKind::LedControl);
            info!(
                target: "booster_interface::rpc",
                sequence = attempt.sequence,
                action = "led_control",
                ?desired_led,
                in_flight = attempt.in_flight_at_start,
                "booster rpc scheduled"
            );
            let operation = match desired_led {
                DesiredLed::Set(_) => "set led color",
                DesiredLed::Stop => "stop led control",
            };
            retryable_rpc_call(
                async move {
                    match desired_led {
                        DesiredLed::Set(color) => {
                            light_control_client
                                .set_led_light_color(color, command.timeout)
                                .await
                        }
                        DesiredLed::Stop => {
                            light_control_client
                                .stop_led_light_control(command.timeout)
                                .await
                        }
                    }
                },
                operation,
                attempt,
            )
            .await
        }
    }));
    sender
}

fn send_retry_command<T: Copy>(
    sender: &watch::Sender<Option<RetryCommand<T>>>,
    target: T,
    timeout: Duration,
    operation: &'static str,
) {
    if sender.send(Some(RetryCommand { target, timeout })).is_err() {
        error!(target: "booster_interface::rpc", operation, "failed to send rpc worker command");
    }
}

fn clear_retry_command<T: Copy>(
    sender: &watch::Sender<Option<RetryCommand<T>>>,
    operation: &'static str,
) {
    if sender.send(None).is_err() {
        error!(target: "booster_interface::rpc", operation, "failed to clear rpc worker command");
    }
}

async fn retryable_rpc_call<T>(
    future: impl Future<Output = Result<T>>,
    operation: impl Into<Cow<'static, str>>,
    attempt: RpcAttempt,
) -> Result<()> {
    if await_rpc_call(future, operation, attempt).await.is_some() {
        Ok(())
    } else {
        color_eyre::eyre::bail!("retryable rpc call failed")
    }
}

async fn await_rpc_call<T>(
    future: impl Future<Output = Result<T>>,
    operation: impl Into<Cow<'static, str>>,
    attempt: RpcAttempt,
) -> Option<T> {
    let operation = operation.into();
    finish_rpc_result(future.await, operation, attempt)
}

async fn await_rpc_call_with_timeout<T>(
    future: impl Future<Output = Result<T>>,
    timeout: Duration,
    operation: impl Into<Cow<'static, str>>,
    attempt: RpcAttempt,
) -> Option<T> {
    let operation = operation.into();
    match tokio::time::timeout(timeout, future).await {
        Ok(result) => finish_rpc_result(result, operation, attempt),
        Err(_) => {
            error!(target: "booster_interface::rpc", operation = %operation, ?timeout, "booster rpc timed out");
            attempt.finish("timeout");
            None
        }
    }
}

fn finish_rpc_result<T>(
    result: Result<T>,
    operation: Cow<'static, str>,
    attempt: RpcAttempt,
) -> Option<T> {
    match result {
        Ok(result) => {
            attempt.finish("ok");
            Some(result)
        }
        Err(error) => {
            let status = if rpc_transport::is_timeout_error(&error) {
                "timeout"
            } else {
                "error"
            };
            if status == "timeout" {
                error!(target: "booster_interface::rpc", operation = %operation, error = %error, "booster rpc timed out");
            } else {
                error!(target: "booster_interface::rpc", operation = %operation, error = %error, "booster rpc failed");
            }
            attempt.finish(status);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn interface_maps_desired_modes_to_robot_modes() {
        assert_eq!(
            robot_mode_for(control::DesiredMode::Damping),
            RobotMode::Damping
        );
        assert_eq!(
            robot_mode_for(control::DesiredMode::Prepare),
            RobotMode::Prepare
        );
        assert_eq!(
            robot_mode_for(control::DesiredMode::Soccer),
            RobotMode::Soccer
        );
    }

    #[test]
    fn rpc_diagnostics_assigns_sequences_and_action_local_in_flight_counts() {
        let diagnostics = std::sync::Arc::new(RpcDiagnostics::default());

        let first_change_mode = diagnostics.begin(RpcActionKind::ChangeMode);
        let second_change_mode = diagnostics.begin(RpcActionKind::ChangeMode);
        let first_move_robot = diagnostics.begin(RpcActionKind::MoveRobot);

        assert_eq!(first_change_mode.sequence, 1);
        assert_eq!(first_change_mode.in_flight_at_start, 1);
        assert_eq!(second_change_mode.sequence, 2);
        assert_eq!(second_change_mode.in_flight_at_start, 2);
        assert_eq!(first_move_robot.sequence, 3);
        assert_eq!(first_move_robot.in_flight_at_start, 1);
    }

    #[test]
    fn rpc_attempt_finish_decrements_action_local_in_flight_count() {
        let diagnostics = std::sync::Arc::new(RpcDiagnostics::default());
        let attempt = diagnostics.begin(RpcActionKind::GetUp);

        assert_eq!(
            diagnostics
                .get_up_in_flight
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        attempt.finish("ok");

        assert_eq!(
            diagnostics
                .get_up_in_flight
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[tokio::test]
    async fn rpc_call_with_timeout_returns_none_on_outer_timeout() {
        let diagnostics = std::sync::Arc::new(RpcDiagnostics::default());
        let attempt = diagnostics.begin(RpcActionKind::GetUp);
        let result = await_rpc_call_with_timeout(
            std::future::pending::<Result<()>>(),
            Duration::from_millis(1),
            "pending test operation",
            attempt,
        )
        .await;

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn rpc_call_waits_for_future_owned_timeout() {
        let diagnostics = std::sync::Arc::new(RpcDiagnostics::default());
        let attempt = diagnostics.begin(RpcActionKind::GetUp);
        let completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let completed_in_future = completed.clone();

        let result = await_rpc_call(
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                completed_in_future.store(true, std::sync::atomic::Ordering::Relaxed);
                Err::<(), _>(color_eyre::eyre::eyre!("inner timeout"))
            },
            "future-owned timeout test operation",
            attempt,
        )
        .await;

        assert!(result.is_none());
        assert!(completed.load(std::sync::atomic::Ordering::Relaxed));
    }
}
