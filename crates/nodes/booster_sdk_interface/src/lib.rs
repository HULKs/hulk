use std::{
    borrow::Cow,
    boxed::Box,
    fmt::Display,
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use booster_sdk::client::{BoosterClient, light_control::LightControlClient};
use color_eyre::{Result, eyre::WrapErr};
use kinematics::joints::head::HeadJoints;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use types::motion_command::MotionCommand;

mod control;
mod kick_transport;

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
    let light_control_client =
        Arc::new(LightControlClient::new().wrap_err("failed to create LightControlClient")?);
    let booster_client = BoosterClient::with_options(booster_rpc_options())
        .wrap_err("failed to create BoosterClient")?;
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
    let booster_client = Arc::new(booster_client);
    let kick_ball_publisher = Arc::new(kick_ball_publisher);
    let rpc_diagnostics = Arc::new(RpcDiagnostics::default());

    tokio::time::sleep(booster_effect_startup_wait()).await;

    let mut state = InterfaceState::new(std::time::Instant::now());
    let mut tick = node.create_timer(std::time::Duration::from_millis(10));

    loop {
        tokio::select! {
            led_command = led_command_sub.recv() => {
                let led_command = led_command?;
                info!(target: "booster_interface::input", ?led_command, "received led command");
                let light_control_client = light_control_client.clone();
                tokio::spawn(handle_led_command(light_control_client, led_command));
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
                        let attempt = rpc_diagnostics.begin(RpcActionKind::VisualKick);
                        info!(
                            target: "booster_interface::rpc",
                            sequence = attempt.sequence,
                            action = "visual_kick",
                            enabled = false,
                            in_flight = attempt.in_flight_at_start,
                            "booster rpc scheduled"
                        );
                        let booster_client = booster_client.clone();
                        tokio::spawn(async move {
                            let _ = await_sdk_call(
                                booster_client.visual_kick(false),
                                timeout,
                                "stop visual kick",
                                attempt,
                            )
                            .await;
                        });
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
                    let mode = sdk_mode_for(desired_mode);
                    let attempt = rpc_diagnostics.begin(RpcActionKind::ChangeMode);
                    info!(
                        target: "booster_interface::rpc",
                        sequence = attempt.sequence,
                        action = "change_mode",
                        ?mode,
                        in_flight = attempt.in_flight_at_start,
                        "booster rpc scheduled"
                    );
                    let booster_client = booster_client.clone();
                    tokio::spawn(async move {
                        let _ = await_sdk_call(
                            booster_client.change_mode(mode),
                            timeout,
                            format!("request booster mode {mode:?}"),
                            attempt,
                        )
                        .await;
                    });
                    state.assumed_mode = desired_mode;
                }

                if motion_kind == MotionCommandKind::StandUp
                    && state.last_motion_kind != MotionCommandKind::StandUp
                    && state.assumed_mode == control::DesiredMode::Prepare
                {
                    let attempt = rpc_diagnostics.begin(RpcActionKind::GetUp);
                    info!(
                        target: "booster_interface::rpc",
                        sequence = attempt.sequence,
                        action = "get_up",
                        in_flight = attempt.in_flight_at_start,
                        "booster rpc scheduled"
                    );
                    let booster_client = booster_client.clone();
                    tokio::spawn(async move {
                        let _ = await_sdk_call(
                            booster_client.get_up(),
                            timeout,
                            "request get_up",
                            attempt,
                        )
                        .await;
                    });
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
                                let _ = await_sdk_call(
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
                        let attempt = rpc_diagnostics.begin(RpcActionKind::VisualKick);
                        info!(
                            target: "booster_interface::rpc",
                            sequence = attempt.sequence,
                            action = "visual_kick",
                            enabled = true,
                            in_flight = attempt.in_flight_at_start,
                            "booster rpc scheduled"
                        );
                        let booster_client = booster_client.clone();
                        tokio::spawn(async move {
                            let _ = await_sdk_call(
                                booster_client.visual_kick(true),
                                timeout,
                                "start visual kick",
                                attempt,
                            )
                            .await;
                        });
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
                    let booster_client = booster_client.clone();
                    tokio::spawn(async move {
                        let _ = await_sdk_call(
                            booster_client.move_robot(step.forward, step.left, step.turn),
                            timeout,
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
                    let booster_client = booster_client.clone();
                    tokio::spawn(async move {
                        let _ = await_sdk_call(
                            booster_client.rotate_head(head_joints.pitch, head_joints.yaw),
                            timeout,
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

async fn handle_led_command(
    light_control_client: Arc<LightControlClient>,
    led_command: LedCommand,
) -> Result<()> {
    match led_command {
        LedCommand::SetParam { r, g, b } => {
            if let Err(err) = light_control_client.set_led_light_color(r, g, b).await {
                error!(target: "booster_interface::led", error = %err, "failed to set leds");
            }
        }
        LedCommand::Stop => {
            if let Err(err) = light_control_client.stop_led_light_control().await {
                error!(target: "booster_interface::led", error = %err, "failed to stop led control");
            }
        }
    };

    Ok(())
}

fn sdk_mode_for(desired_mode: control::DesiredMode) -> booster_sdk::types::RobotMode {
    match desired_mode {
        control::DesiredMode::Damping => booster_sdk::types::RobotMode::Damping,
        control::DesiredMode::Prepare => booster_sdk::types::RobotMode::Prepare,
        control::DesiredMode::Soccer => booster_sdk::types::RobotMode::Soccer,
    }
}

fn booster_rpc_options() -> booster_sdk::dds::RpcClientOptions {
    booster_sdk::dds::RpcClientOptions::default().without_startup_wait()
}

fn booster_effect_startup_wait() -> Duration {
    booster_sdk::dds::RpcClientOptions::default().startup_wait
}

async fn await_sdk_call<T, E>(
    future: impl Future<Output = std::result::Result<T, E>>,
    timeout: Duration,
    operation: impl Into<Cow<'static, str>>,
    attempt: RpcAttempt,
) -> Option<T>
where
    E: Display,
{
    let operation = operation.into();
    match tokio::time::timeout(timeout, future).await {
        Ok(Ok(result)) => {
            attempt.finish("ok");
            Some(result)
        }
        Ok(Err(error)) => {
            error!(target: "booster_interface::rpc", operation = %operation, error = %error, "booster rpc failed");
            attempt.finish("error");
            None
        }
        Err(_) => {
            error!(target: "booster_interface::rpc", operation = %operation, ?timeout, "booster rpc timed out");
            attempt.finish("timeout");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn interface_parameters() -> Parameters {
        Parameters {
            walking: WalkingParameters {
                hybrid_align_distance: 1.0,
                max_alignment_rate: 1.0,
                deceleration_distance: 0.5,
            },
            move_robot_message_interval: Duration::from_millis(33),
            kicking: types::parameters::BoosterKickingParameters {
                kick_message_interval: Duration::from_millis(33),
                kick_power: types::parameters::KickPowerParameters {
                    rumpelstilzchen: 1.5,
                    schlong: 6.0,
                },
            },
            rotate_head_message_interval: Duration::from_millis(33),
            sdk_request_timeout: Duration::from_millis(100),
        }
    }

    #[test]
    fn interface_maps_desired_modes_to_sdk_modes() {
        assert_eq!(
            sdk_mode_for(control::DesiredMode::Damping),
            booster_sdk::types::RobotMode::Damping
        );
        assert_eq!(
            sdk_mode_for(control::DesiredMode::Prepare),
            booster_sdk::types::RobotMode::Prepare
        );
        assert_eq!(
            sdk_mode_for(control::DesiredMode::Soccer),
            booster_sdk::types::RobotMode::Soccer
        );
    }

    #[test]
    fn booster_rpc_options_disable_internal_startup_wait() {
        assert_eq!(booster_rpc_options().startup_wait, Duration::ZERO);
    }

    #[test]
    fn booster_loop_keeps_sdk_startup_wait_outside_rpc_timeout() {
        assert_eq!(
            booster_effect_startup_wait(),
            booster_sdk::dds::RpcClientOptions::default().startup_wait
        );
        assert!(booster_effect_startup_wait() > interface_parameters().sdk_request_timeout);
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
    async fn sdk_call_returns_none_on_timeout() {
        let diagnostics = std::sync::Arc::new(RpcDiagnostics::default());
        let attempt = diagnostics.begin(RpcActionKind::GetUp);
        let result = await_sdk_call(
            std::future::pending::<std::result::Result<(), std::convert::Infallible>>(),
            Duration::from_millis(1),
            "pending test operation",
            attempt,
        )
        .await;

        assert!(result.is_none());
    }
}
