use std::{
    borrow::Cow, boxed::Box, fmt::Display, future::Future, pin::Pin, sync::Arc, time::Duration,
};

use booster_sdk::client::{BoosterClient, light_control::LightControlClient};
use color_eyre::{Result, eyre::WrapErr};
use kinematics::joints::head::HeadJoints;
use ros_z::{prelude::*, time::Clock};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};
use types::{
    buttons::{ButtonPressType, Buttons},
    motion_command::MotionCommand,
};

mod control;
mod kick_transport;

const MOTION_COMMAND_TOPIC: &str = "behavior/motion_command";
const EFFECT_TASK_RESULT_CHANNEL_CAPACITY: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct WalkingParameters {
    pub hybrid_align_distance: f32,
    pub max_alignment_rate: f32,
    pub deceleration_distance: f32,
}

#[derive(Serialize, Deserialize, Message)]
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
    pub mode_poll_interval: std::time::Duration,
    pub mode_retry_interval: std::time::Duration,
    pub stand_up_retry_interval: std::time::Duration,
    pub visual_kick_retry_interval: std::time::Duration,
    pub remote_stop_toggle: bool,
}

#[derive(Clone)]
struct EffectInputs {
    motion_command: Arc<MotionCommand>,
    head_joints: Option<HeadJoints<f32>>,
    emergency_damping: bool,
    parameters: Parameters,
}

struct InterfaceState {
    confirmed_mode: Option<booster_sdk::types::RobotMode>,
    last_requested_mode: Option<control::DesiredMode>,
    mode_poll_in_flight: bool,
    mode_request: ModeRequestState,
    next_mode_request_generation: u64,
    last_mode_request: std::time::Instant,
    last_mode_poll: std::time::Instant,
    last_move_robot: std::time::Instant,
    last_rotate_head: std::time::Instant,
    last_kick: std::time::Instant,
    last_visual_kick_attempt: Option<std::time::Instant>,
    visual_kick: control::VisualKickState,
    visual_kick_in_flight: Option<VisualKickRequest>,
    stand_up_request: StandUpRequestState,
    move_robot_in_flight: bool,
    rotate_head_in_flight: bool,
    kick_publish_in_flight: bool,
}

impl Default for InterfaceState {
    fn default() -> Self {
        let now = std::time::Instant::now();
        Self {
            confirmed_mode: None,
            last_requested_mode: None,
            mode_poll_in_flight: false,
            mode_request: ModeRequestState::Idle,
            next_mode_request_generation: 0,
            last_mode_request: now,
            last_mode_poll: now,
            last_move_robot: now,
            last_rotate_head: now,
            last_kick: now,
            last_visual_kick_attempt: None,
            visual_kick: control::VisualKickState::default(),
            visual_kick_in_flight: None,
            stand_up_request: StandUpRequestState::default(),
            move_robot_in_flight: false,
            rotate_head_in_flight: false,
            kick_publish_in_flight: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ModeRequestState {
    Idle,
    InFlight {
        generation: u64,
        desired_mode: control::DesiredMode,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum StandUpRequestState {
    Idle {
        last_command_was_stand_up: bool,
        next_generation: u64,
    },
    Pending {
        generation: u64,
        last_attempt: Option<std::time::Instant>,
    },
    InFlight {
        generation: u64,
        last_attempt: std::time::Instant,
        command_still_stand_up: bool,
    },
}

impl Default for StandUpRequestState {
    fn default() -> Self {
        Self::Idle {
            last_command_was_stand_up: false,
            next_generation: 0,
        }
    }
}

impl StandUpRequestState {
    fn update_command(&mut self, command: &MotionCommand) {
        let is_stand_up = matches!(command, MotionCommand::StandUp);

        *self = match *self {
            StandUpRequestState::Idle {
                last_command_was_stand_up,
                next_generation,
            } => {
                if is_stand_up && !last_command_was_stand_up {
                    StandUpRequestState::Pending {
                        generation: next_generation,
                        last_attempt: None,
                    }
                } else {
                    StandUpRequestState::Idle {
                        last_command_was_stand_up: is_stand_up,
                        next_generation,
                    }
                }
            }
            StandUpRequestState::Pending {
                generation,
                last_attempt,
            } => {
                if is_stand_up {
                    StandUpRequestState::Pending {
                        generation,
                        last_attempt,
                    }
                } else {
                    StandUpRequestState::Idle {
                        last_command_was_stand_up: false,
                        next_generation: generation + 1,
                    }
                }
            }
            StandUpRequestState::InFlight {
                generation,
                last_attempt,
                ..
            } => StandUpRequestState::InFlight {
                generation,
                last_attempt,
                command_still_stand_up: is_stand_up,
            },
        };
    }

    #[cfg(test)]
    fn is_pending(&self) -> bool {
        matches!(self, StandUpRequestState::Pending { .. })
    }

    #[cfg(test)]
    fn is_in_flight(&self) -> bool {
        matches!(self, StandUpRequestState::InFlight { .. })
    }

    fn should_request(
        &self,
        confirmed_mode: Option<booster_sdk::types::RobotMode>,
        now: std::time::Instant,
        retry_interval: std::time::Duration,
        allow_stand_up: bool,
    ) -> bool {
        let StandUpRequestState::Pending { last_attempt, .. } = *self else {
            return false;
        };

        allow_stand_up
            && matches!(
                confirmed_mode,
                Some(
                    booster_sdk::types::RobotMode::Prepare | booster_sdk::types::RobotMode::Damping
                )
            )
            && last_attempt.is_none_or(|last_attempt| {
                now.saturating_duration_since(last_attempt) >= retry_interval
            })
    }

    fn record_attempt(&mut self, now: std::time::Instant) -> Option<u64> {
        let StandUpRequestState::Pending { generation, .. } = *self else {
            return None;
        };

        *self = StandUpRequestState::InFlight {
            generation,
            last_attempt: now,
            command_still_stand_up: true,
        };

        Some(generation)
    }

    fn record_completion(&mut self, generation: u64, success: bool) {
        let StandUpRequestState::InFlight {
            generation: current_generation,
            last_attempt,
            command_still_stand_up,
        } = *self
        else {
            return;
        };

        if current_generation != generation {
            return;
        }

        if success || !command_still_stand_up {
            *self = StandUpRequestState::Idle {
                last_command_was_stand_up: command_still_stand_up,
                next_generation: generation + 1,
            };
        } else {
            *self = StandUpRequestState::Pending {
                generation,
                last_attempt: Some(last_attempt),
            };
        }
    }
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
    let booster_client = BoosterClient::new().wrap_err("failed to create BoosterClient")?;
    let kick_ball_publisher = kick_transport::KickBallPublisher::new(ctx.session())
        .await
        .wrap_err("failed to create kick ball publisher")?;

    let motion_command_cache = node
        .create_cache::<MotionCommand>(MOTION_COMMAND_TOPIC, 1)
        .wrap_err("failed to create motion_command cache")?
        .build()
        .await
        .wrap_err("failed to build motion_command cache")?;
    let head_joints_cache = node
        .create_cache::<HeadJoints<f32>>("head_joints_command", 1)
        .wrap_err("failed to create head_joints_command cache")?
        .build()
        .await
        .wrap_err("failed to build head_joints_command cache")?;
    let led_command_sub = node
        .subscriber::<LedCommand>("commands/led_command")
        .wrap_err("failed to create commands/led_command subscriber")?
        .build()
        .await
        .wrap_err("failed to build commands/led_command subscriber")?;
    let buttons_sub = node
        .subscriber::<Buttons<Option<ButtonPressType>>>("buttons")
        .wrap_err("failed to create buttons subscriber")?
        .build()
        .await
        .wrap_err("failed to build buttons subscriber")?;

    let initial_parameters = parameters.snapshot().typed().clone();
    let default_motion_command = Arc::new(MotionCommand::Damping);
    let (effect_inputs_tx, effect_inputs_rx) = watch::channel(EffectInputs {
        motion_command: default_motion_command.clone(),
        head_joints: None,
        emergency_damping: false,
        parameters: initial_parameters,
    });
    tokio::spawn(run_effect_worker(
        effect_inputs_rx,
        booster_client,
        kick_ball_publisher,
        node.clock().clone(),
    ));

    let mut local_stop_toggle = false;
    let mut tick = node.create_timer(std::time::Duration::from_millis(10));

    loop {
        tokio::select! {
            led_command = led_command_sub.recv() => {
                let light_control_client = light_control_client.clone();
                tokio::spawn(handle_led_command(light_control_client, led_command?));
            }
            buttons = buttons_sub.recv() => {
                let buttons = buttons?;
                if button_requests_local_stop_toggle(&buttons) {
                    local_stop_toggle = !local_stop_toggle;
                }
            }
            _ = tick.tick() => {
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();
                let latest_motion_command = motion_command_cache
                    .get_latest()
                    .unwrap_or_else(|| default_motion_command.clone());
                let latest_head_joints = head_joints_cache
                    .get_latest()
                    .map(|head_joints| *head_joints);
                let emergency_damping = local_stop_toggle != parameters.remote_stop_toggle;
                effect_inputs_tx.send_replace(EffectInputs {
                    motion_command: latest_motion_command,
                    head_joints: latest_head_joints,
                    emergency_damping,
                    parameters: parameters.clone(),
                });
            }
        }
    }
}

async fn run_effect_worker(
    mut effect_inputs_rx: watch::Receiver<EffectInputs>,
    booster_client: BoosterClient,
    kick_ball_publisher: kick_transport::KickBallPublisher,
    clock: Clock,
) {
    let booster_client = Arc::new(booster_client);
    let kick_ball_publisher = Arc::new(kick_ball_publisher);
    let (effect_result_tx, mut effect_result_rx) =
        mpsc::channel(EFFECT_TASK_RESULT_CHANNEL_CAPACITY);
    let mut state = InterfaceState::default();

    loop {
        tokio::select! {
            result = effect_result_rx.recv() => {
                if let Some(result) = result {
                    apply_effect_task_result(&mut state, result);
                }
            }
            changed = effect_inputs_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                drain_effect_task_results(&mut state, &mut effect_result_rx);
                let inputs = effect_inputs_rx.borrow_and_update().clone();

                drive_booster_effects(
                    &mut state,
                    booster_client.clone(),
                    kick_ball_publisher.clone(),
                    effect_result_tx.clone(),
                    &clock,
                    &inputs,
                );
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
                log::error!("failed to set leds: {err}");
            }
        }
        LedCommand::Stop => {
            if let Err(err) = light_control_client.stop_led_light_control().await {
                log::error!("failed to stop led control: {err}");
            }
        }
    };

    Ok(())
}

fn sdk_mode_for(desired_mode: control::DesiredMode) -> booster_sdk::types::RobotMode {
    match desired_mode {
        control::DesiredMode::Damping => booster_sdk::types::RobotMode::Damping,
        control::DesiredMode::Prepare => booster_sdk::types::RobotMode::Prepare,
        control::DesiredMode::Walking => booster_sdk::types::RobotMode::Walking,
    }
}

fn button_requests_local_stop_toggle(buttons: &Buttons<Option<ButtonPressType>>) -> bool {
    matches!(buttons.f1, Some(ButtonPressType::Short))
        || matches!(buttons.stand, Some(ButtonPressType::Short))
}

fn visual_kick_transition_for(
    state: control::VisualKickState,
    should_be_active: bool,
) -> control::VisualKickTransition {
    match (state.is_active(), should_be_active) {
        (false, true) => control::VisualKickTransition::Start,
        (true, false) => control::VisualKickTransition::Stop,
        _ => control::VisualKickTransition::None,
    }
}

fn visual_kick_retry_due(
    last_attempt: Option<std::time::Instant>,
    now: std::time::Instant,
    retry_interval: std::time::Duration,
) -> bool {
    last_attempt
        .is_none_or(|last_attempt| now.saturating_duration_since(last_attempt) >= retry_interval)
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum VisualKickRequest {
    Start,
    Stop,
}

impl VisualKickRequest {
    fn active(self) -> bool {
        matches!(self, VisualKickRequest::Start)
    }

    fn operation(self) -> &'static str {
        match self {
            VisualKickRequest::Start => "start visual kick",
            VisualKickRequest::Stop => "stop visual kick",
        }
    }
}

enum EffectTaskResult {
    ModePoll(Option<booster_sdk::types::RobotMode>),
    ModeRequest {
        generation: u64,
        desired_mode: control::DesiredMode,
    },
    StandUp {
        generation: u64,
        success: bool,
    },
    VisualKick {
        request: VisualKickRequest,
        success: bool,
    },
    MoveRobot,
    RotateHead,
    KickPublish,
}

fn mode_poll_due(state: &InterfaceState, now: std::time::Instant, parameters: &Parameters) -> bool {
    !state.mode_poll_in_flight
        && now.duration_since(state.last_mode_poll) >= parameters.mode_poll_interval
}

fn mark_mode_poll_started(state: &mut InterfaceState, now: std::time::Instant) {
    state.mode_poll_in_flight = true;
    state.last_mode_poll = now;
}

fn mode_request_due(
    state: &InterfaceState,
    desired_mode: control::DesiredMode,
    now: std::time::Instant,
    parameters: &Parameters,
) -> bool {
    if matches!(
        state.mode_request,
        ModeRequestState::InFlight {
            desired_mode: in_flight_mode,
            ..
        } if desired_mode == control::DesiredMode::Damping
            && in_flight_mode != control::DesiredMode::Damping
    ) {
        return true;
    }

    let confirmed_desired_mode = state.confirmed_mode == Some(sdk_mode_for(desired_mode));
    if confirmed_desired_mode {
        return false;
    }

    match state.mode_request {
        ModeRequestState::Idle => {
            state.last_requested_mode != Some(desired_mode)
                || now.duration_since(state.last_mode_request) >= parameters.mode_retry_interval
        }
        ModeRequestState::InFlight { .. } => false,
    }
}

fn mark_mode_request_started(
    state: &mut InterfaceState,
    desired_mode: control::DesiredMode,
    now: std::time::Instant,
) -> u64 {
    let generation = state.next_mode_request_generation;
    state.next_mode_request_generation += 1;
    state.mode_request = ModeRequestState::InFlight {
        generation,
        desired_mode,
    };
    state.last_requested_mode = Some(desired_mode);
    state.last_mode_request = now;
    generation
}

fn apply_effect_task_result(state: &mut InterfaceState, result: EffectTaskResult) {
    match result {
        EffectTaskResult::ModePoll(mode) => {
            state.confirmed_mode = mode;
            state.mode_poll_in_flight = false;
        }
        EffectTaskResult::ModeRequest {
            generation,
            desired_mode,
        } => {
            if matches!(
                state.mode_request,
                ModeRequestState::InFlight {
                    generation: in_flight_generation,
                    desired_mode: in_flight_mode,
                } if in_flight_generation == generation && in_flight_mode == desired_mode
            ) {
                state.mode_request = ModeRequestState::Idle;
            }
        }
        EffectTaskResult::StandUp {
            generation,
            success,
        } => {
            state
                .stand_up_request
                .record_completion(generation, success);
        }
        EffectTaskResult::VisualKick { request, success } => {
            if state.visual_kick_in_flight == Some(request) {
                state.visual_kick_in_flight = None;
                if success {
                    state.visual_kick.update(request.active());
                    state.last_visual_kick_attempt = None;
                }
            }
        }
        EffectTaskResult::MoveRobot => {
            state.move_robot_in_flight = false;
        }
        EffectTaskResult::RotateHead => {
            state.rotate_head_in_flight = false;
        }
        EffectTaskResult::KickPublish => {
            state.kick_publish_in_flight = false;
        }
    }
}

fn drain_effect_task_results(
    state: &mut InterfaceState,
    effect_result_rx: &mut mpsc::Receiver<EffectTaskResult>,
) {
    while let Ok(result) = effect_result_rx.try_recv() {
        apply_effect_task_result(state, result);
    }
}

fn spawn_mode_poll_if_due(
    state: &mut InterfaceState,
    booster_client: Arc<BoosterClient>,
    effect_result_tx: mpsc::Sender<EffectTaskResult>,
    now: std::time::Instant,
    parameters: &Parameters,
) {
    if !mode_poll_due(state, now, parameters) {
        return;
    }

    mark_mode_poll_started(state, now);
    let timeout = parameters.sdk_request_timeout;
    tokio::spawn(async move {
        let mode = poll_mode(&booster_client, timeout).await;
        let _ = effect_result_tx
            .send(EffectTaskResult::ModePoll(mode))
            .await;
    });
}

fn spawn_mode_request_if_due(
    state: &mut InterfaceState,
    booster_client: Arc<BoosterClient>,
    effect_result_tx: mpsc::Sender<EffectTaskResult>,
    desired_mode: control::DesiredMode,
    now: std::time::Instant,
    parameters: &Parameters,
) {
    if !mode_request_due(state, desired_mode, now, parameters) {
        return;
    }

    let generation = mark_mode_request_started(state, desired_mode, now);
    let timeout = parameters.sdk_request_timeout;
    tokio::spawn(async move {
        request_mode(&booster_client, desired_mode, timeout).await;
        let _ = effect_result_tx
            .send(EffectTaskResult::ModeRequest {
                generation,
                desired_mode,
            })
            .await;
    });
}

fn mark_stand_up_started(state: &mut InterfaceState, now: std::time::Instant) -> Option<u64> {
    state.stand_up_request.record_attempt(now)
}

fn spawn_stand_up_if_due(
    state: &mut InterfaceState,
    booster_client: Arc<BoosterClient>,
    effect_result_tx: mpsc::Sender<EffectTaskResult>,
    now: std::time::Instant,
    parameters: &Parameters,
    allow_stand_up: bool,
) {
    if !state.stand_up_request.should_request(
        state.confirmed_mode,
        now,
        parameters.stand_up_retry_interval,
        allow_stand_up,
    ) {
        return;
    }

    let Some(generation) = mark_stand_up_started(state, now) else {
        return;
    };
    let timeout = parameters.sdk_request_timeout;
    tokio::spawn(async move {
        let success = await_sdk_call(booster_client.get_up(), timeout, "request get_up")
            .await
            .is_some();
        let _ = effect_result_tx
            .send(EffectTaskResult::StandUp {
                generation,
                success,
            })
            .await;
    });
}

fn mark_visual_kick_started(
    state: &mut InterfaceState,
    request: VisualKickRequest,
    now: std::time::Instant,
) {
    state.visual_kick_in_flight = Some(request);
    state.last_visual_kick_attempt = Some(now);
}

fn spawn_visual_kick_if_due(
    state: &mut InterfaceState,
    booster_client: Arc<BoosterClient>,
    effect_result_tx: mpsc::Sender<EffectTaskResult>,
    request: VisualKickRequest,
    now: std::time::Instant,
    parameters: &Parameters,
) {
    if state.visual_kick_in_flight.is_some()
        || !visual_kick_retry_due(
            state.last_visual_kick_attempt,
            now,
            parameters.visual_kick_retry_interval,
        )
    {
        return;
    }

    mark_visual_kick_started(state, request, now);
    let timeout = parameters.sdk_request_timeout;
    tokio::spawn(async move {
        let success = await_sdk_call(
            booster_client.visual_kick(request.active()),
            timeout,
            request.operation(),
        )
        .await
        .is_some();
        let _ = effect_result_tx
            .send(EffectTaskResult::VisualKick { request, success })
            .await;
    });
}

fn mark_move_robot_started(state: &mut InterfaceState, now: std::time::Instant) {
    state.move_robot_in_flight = true;
    state.last_move_robot = now;
}

fn spawn_move_robot_if_due(
    state: &mut InterfaceState,
    booster_client: Arc<BoosterClient>,
    effect_result_tx: mpsc::Sender<EffectTaskResult>,
    step: types::step::Step,
    now: std::time::Instant,
    parameters: &Parameters,
) {
    if state.move_robot_in_flight
        || now.duration_since(state.last_move_robot) < parameters.move_robot_message_interval
    {
        return;
    }

    mark_move_robot_started(state, now);
    let timeout = parameters.sdk_request_timeout;
    tokio::spawn(async move {
        let _ = await_sdk_call(
            booster_client.move_robot(step.forward, step.left, step.turn),
            timeout,
            "send move_robot",
        )
        .await;
        let _ = effect_result_tx.send(EffectTaskResult::MoveRobot).await;
    });
}

fn mark_rotate_head_started(state: &mut InterfaceState, now: std::time::Instant) {
    state.rotate_head_in_flight = true;
    state.last_rotate_head = now;
}

fn spawn_rotate_head_if_due(
    state: &mut InterfaceState,
    booster_client: Arc<BoosterClient>,
    effect_result_tx: mpsc::Sender<EffectTaskResult>,
    head_joints: HeadJoints<f32>,
    now: std::time::Instant,
    parameters: &Parameters,
) {
    if state.rotate_head_in_flight
        || now.duration_since(state.last_rotate_head) < parameters.rotate_head_message_interval
    {
        return;
    }

    mark_rotate_head_started(state, now);
    let timeout = parameters.sdk_request_timeout;
    tokio::spawn(async move {
        let _ = await_sdk_call(
            booster_client.rotate_head(head_joints.pitch, head_joints.yaw),
            timeout,
            "rotate head",
        )
        .await;
        let _ = effect_result_tx.send(EffectTaskResult::RotateHead).await;
    });
}

fn mark_kick_publish_started(state: &mut InterfaceState, now: std::time::Instant) {
    state.kick_publish_in_flight = true;
    state.last_kick = now;
}

fn spawn_kick_publish_if_due(
    state: &mut InterfaceState,
    kick_ball_publisher: Arc<kick_transport::KickBallPublisher>,
    effect_result_tx: mpsc::Sender<EffectTaskResult>,
    kick: booster::Kick,
    now: std::time::Instant,
    parameters: &Parameters,
) {
    if state.kick_publish_in_flight
        || now.duration_since(state.last_kick) < parameters.kicking.kick_message_interval
    {
        return;
    }

    mark_kick_publish_started(state, now);
    let timeout = parameters.sdk_request_timeout;
    tokio::spawn(async move {
        let _ = await_sdk_call(
            kick_ball_publisher.publish(&kick),
            timeout,
            "publish visual kick command",
        )
        .await;
        let _ = effect_result_tx.send(EffectTaskResult::KickPublish).await;
    });
}

async fn await_sdk_call<T, E>(
    future: impl Future<Output = std::result::Result<T, E>>,
    timeout: Duration,
    operation: impl Into<Cow<'static, str>>,
) -> Option<T>
where
    E: Display,
{
    let operation = operation.into();
    match tokio::time::timeout(timeout, future).await {
        Ok(Ok(result)) => Some(result),
        Ok(Err(error)) => {
            log::error!("failed to {operation}: {error}");
            None
        }
        Err(_) => {
            log::error!("timed out while trying to {operation} after {timeout:?}");
            None
        }
    }
}

fn robot_mode_from_raw_id(raw_mode: i32) -> Option<booster_sdk::types::RobotMode> {
    match booster_sdk::types::RobotMode::try_from(raw_mode) {
        Ok(mode) => Some(mode),
        Err(_) => {
            log::warn!("unrecognized booster mode id: {raw_mode}");
            None
        }
    }
}

async fn poll_mode(
    client: &BoosterClient,
    timeout: Duration,
) -> Option<booster_sdk::types::RobotMode> {
    let mode = await_sdk_call(client.get_mode(), timeout, "poll booster mode").await?;
    robot_mode_from_raw_id(mode.mode)
}

async fn request_mode(
    client: &BoosterClient,
    desired_mode: control::DesiredMode,
    timeout: Duration,
) {
    let mode = sdk_mode_for(desired_mode);
    let _ = await_sdk_call(
        client.change_mode(mode),
        timeout,
        format!("request booster mode {mode:?}"),
    )
    .await;
}

fn drive_booster_effects(
    state: &mut InterfaceState,
    booster_client: Arc<BoosterClient>,
    kick_ball_publisher: Arc<kick_transport::KickBallPublisher>,
    effect_result_tx: mpsc::Sender<EffectTaskResult>,
    clock: &Clock,
    inputs: &EffectInputs,
) {
    let latest_head_joints = inputs.head_joints;
    let emergency_damping = inputs.emergency_damping;
    let parameters = &inputs.parameters;
    let motion_command = inputs.motion_command.as_ref();
    let mut now = std::time::Instant::now();

    spawn_mode_poll_if_due(
        state,
        booster_client.clone(),
        effect_result_tx.clone(),
        now,
        parameters,
    );

    let desired_mode = control::desired_mode_for(motion_command, emergency_damping);
    spawn_mode_request_if_due(
        state,
        booster_client.clone(),
        effect_result_tx.clone(),
        desired_mode,
        now,
        parameters,
    );

    now = std::time::Instant::now();

    let walking_allowed =
        control::confirmed_mode_allows_walking(state.confirmed_mode) && !emergency_damping;

    state.stand_up_request.update_command(motion_command);
    spawn_stand_up_if_due(
        state,
        booster_client.clone(),
        effect_result_tx.clone(),
        now,
        parameters,
        !emergency_damping,
    );

    if !walking_allowed {
        let transition = visual_kick_transition_for(state.visual_kick, false);
        if transition == control::VisualKickTransition::Stop {
            spawn_visual_kick_if_due(
                state,
                booster_client,
                effect_result_tx,
                VisualKickRequest::Stop,
                now,
                parameters,
            );
        } else if transition == control::VisualKickTransition::None {
            state.last_visual_kick_attempt = None;
        }
        return;
    }

    let step = control::step_from_motion_command(motion_command, &parameters.walking);
    spawn_move_robot_if_due(
        state,
        booster_client.clone(),
        effect_result_tx.clone(),
        step,
        now,
        parameters,
    );

    if let Some(head_joints) = latest_head_joints {
        spawn_rotate_head_if_due(
            state,
            booster_client.clone(),
            effect_result_tx.clone(),
            head_joints,
            now,
            parameters,
        );
    }

    let should_visual_kick = matches!(motion_command, MotionCommand::VisualKick { .. });
    match visual_kick_transition_for(state.visual_kick, should_visual_kick) {
        control::VisualKickTransition::Start => {
            spawn_visual_kick_if_due(
                state,
                booster_client.clone(),
                effect_result_tx.clone(),
                VisualKickRequest::Start,
                now,
                parameters,
            );
        }
        control::VisualKickTransition::Stop => {
            spawn_visual_kick_if_due(
                state,
                booster_client,
                effect_result_tx.clone(),
                VisualKickRequest::Stop,
                now,
                parameters,
            );
        }
        control::VisualKickTransition::None => {
            state.last_visual_kick_attempt = None;
        }
    }

    if should_visual_kick
        && let Some(kick) =
            control::kick_from_motion_command(motion_command, clock.now(), &parameters.kicking)
    {
        spawn_kick_publish_if_due(
            state,
            kick_ball_publisher,
            effect_result_tx,
            kick,
            now,
            parameters,
        );
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
            mode_poll_interval: Duration::from_millis(100),
            mode_retry_interval: Duration::from_millis(500),
            stand_up_retry_interval: Duration::from_millis(500),
            visual_kick_retry_interval: Duration::from_millis(500),
            remote_stop_toggle: false,
        }
    }

    #[tokio::test]
    async fn draining_effect_task_results_applies_ready_results() {
        let (tx, mut rx) = mpsc::channel(2);
        let generation = 0;
        tx.send(EffectTaskResult::ModePoll(Some(
            booster_sdk::types::RobotMode::Walking,
        )))
        .await
        .unwrap();
        tx.send(EffectTaskResult::ModeRequest {
            generation,
            desired_mode: control::DesiredMode::Walking,
        })
        .await
        .unwrap();

        let mut state = InterfaceState {
            mode_poll_in_flight: true,
            mode_request: ModeRequestState::InFlight {
                generation,
                desired_mode: control::DesiredMode::Walking,
            },
            ..Default::default()
        };

        drain_effect_task_results(&mut state, &mut rx);

        assert_eq!(
            state.confirmed_mode,
            Some(booster_sdk::types::RobotMode::Walking)
        );
        assert!(!state.mode_poll_in_flight);
        assert_eq!(state.mode_request, ModeRequestState::Idle);
    }

    #[test]
    fn mode_poll_lifecycle_blocks_duplicate_polls_until_completion() {
        let parameters = interface_parameters();
        let mut state = InterfaceState::default();
        let due = state.last_mode_poll + parameters.mode_poll_interval;

        assert!(mode_poll_due(&state, due, &parameters));

        mark_mode_poll_started(&mut state, due);

        assert!(state.mode_poll_in_flight);
        assert_eq!(state.last_mode_poll, due);
        assert!(!mode_poll_due(
            &state,
            due + parameters.mode_poll_interval,
            &parameters,
        ));

        apply_effect_task_result(
            &mut state,
            EffectTaskResult::ModePoll(Some(booster_sdk::types::RobotMode::Walking)),
        );

        assert!(!state.mode_poll_in_flight);
        assert_eq!(
            state.confirmed_mode,
            Some(booster_sdk::types::RobotMode::Walking)
        );
    }

    #[test]
    fn mode_request_lifecycle_blocks_duplicate_requests_until_completion() {
        let parameters = interface_parameters();
        let mut state = InterfaceState {
            confirmed_mode: Some(booster_sdk::types::RobotMode::Prepare),
            ..Default::default()
        };
        let due = state.last_mode_request + parameters.mode_retry_interval;
        let desired_mode = control::DesiredMode::Walking;

        assert!(mode_request_due(&state, desired_mode, due, &parameters));

        let generation = mark_mode_request_started(&mut state, desired_mode, due);

        assert_eq!(
            state.mode_request,
            ModeRequestState::InFlight {
                generation,
                desired_mode,
            }
        );
        assert_eq!(state.last_requested_mode, Some(desired_mode));
        assert_eq!(state.last_mode_request, due);
        assert!(!mode_request_due(
            &state,
            desired_mode,
            due + parameters.mode_retry_interval,
            &parameters,
        ));

        apply_effect_task_result(
            &mut state,
            EffectTaskResult::ModeRequest {
                generation,
                desired_mode,
            },
        );

        assert_eq!(state.mode_request, ModeRequestState::Idle);
    }

    #[test]
    fn damping_request_supersedes_in_flight_walking_request() {
        let parameters = interface_parameters();
        let mut state = InterfaceState {
            confirmed_mode: Some(booster_sdk::types::RobotMode::Prepare),
            ..Default::default()
        };
        let due = state.last_mode_request + parameters.mode_retry_interval;

        let walking_generation =
            mark_mode_request_started(&mut state, control::DesiredMode::Walking, due);

        assert!(mode_request_due(
            &state,
            control::DesiredMode::Damping,
            due,
            &parameters,
        ));

        let damping_generation =
            mark_mode_request_started(&mut state, control::DesiredMode::Damping, due);

        assert_ne!(walking_generation, damping_generation);
        assert_eq!(
            state.mode_request,
            ModeRequestState::InFlight {
                generation: damping_generation,
                desired_mode: control::DesiredMode::Damping,
            }
        );

        apply_effect_task_result(
            &mut state,
            EffectTaskResult::ModeRequest {
                generation: walking_generation,
                desired_mode: control::DesiredMode::Walking,
            },
        );

        assert_eq!(
            state.mode_request,
            ModeRequestState::InFlight {
                generation: damping_generation,
                desired_mode: control::DesiredMode::Damping,
            }
        );
    }

    #[test]
    fn damping_request_supersedes_walking_request_even_when_damping_is_confirmed() {
        let parameters = interface_parameters();
        let mut state = InterfaceState {
            confirmed_mode: Some(booster_sdk::types::RobotMode::Damping),
            ..Default::default()
        };
        let due = state.last_mode_request + parameters.mode_retry_interval;
        let walking_generation =
            mark_mode_request_started(&mut state, control::DesiredMode::Walking, due);

        assert!(mode_request_due(
            &state,
            control::DesiredMode::Damping,
            due,
            &parameters,
        ));

        let damping_generation =
            mark_mode_request_started(&mut state, control::DesiredMode::Damping, due);

        assert_ne!(walking_generation, damping_generation);
    }

    #[test]
    fn matching_damping_completion_clears_in_flight_mode_request() {
        let mut state = InterfaceState::default();
        let generation = mark_mode_request_started(
            &mut state,
            control::DesiredMode::Damping,
            std::time::Instant::now(),
        );

        apply_effect_task_result(
            &mut state,
            EffectTaskResult::ModeRequest {
                generation,
                desired_mode: control::DesiredMode::Damping,
            },
        );

        assert_eq!(state.mode_request, ModeRequestState::Idle);
    }

    #[test]
    fn stale_mode_request_completion_does_not_clear_newer_request() {
        let stale_generation = 0;
        let in_flight_generation = 1;
        let mut state = InterfaceState {
            mode_request: ModeRequestState::InFlight {
                generation: in_flight_generation,
                desired_mode: control::DesiredMode::Damping,
            },
            ..Default::default()
        };

        apply_effect_task_result(
            &mut state,
            EffectTaskResult::ModeRequest {
                generation: stale_generation,
                desired_mode: control::DesiredMode::Damping,
            },
        );

        assert_eq!(
            state.mode_request,
            ModeRequestState::InFlight {
                generation: in_flight_generation,
                desired_mode: control::DesiredMode::Damping,
            }
        );
    }

    #[test]
    fn move_robot_lifecycle_blocks_duplicate_work_until_completion() {
        let mut state = InterfaceState::default();

        assert!(!state.move_robot_in_flight);

        mark_move_robot_started(&mut state, std::time::Instant::now());

        assert!(state.move_robot_in_flight);

        apply_effect_task_result(&mut state, EffectTaskResult::MoveRobot);

        assert!(!state.move_robot_in_flight);
    }

    #[test]
    fn rotate_head_lifecycle_blocks_duplicate_work_until_completion() {
        let mut state = InterfaceState::default();

        assert!(!state.rotate_head_in_flight);

        mark_rotate_head_started(&mut state, std::time::Instant::now());

        assert!(state.rotate_head_in_flight);

        apply_effect_task_result(&mut state, EffectTaskResult::RotateHead);

        assert!(!state.rotate_head_in_flight);
    }

    #[test]
    fn kick_publish_lifecycle_blocks_duplicate_work_until_completion() {
        let mut state = InterfaceState::default();

        assert!(!state.kick_publish_in_flight);

        mark_kick_publish_started(&mut state, std::time::Instant::now());

        assert!(state.kick_publish_in_flight);

        apply_effect_task_result(&mut state, EffectTaskResult::KickPublish);

        assert!(!state.kick_publish_in_flight);
    }

    #[test]
    fn stand_up_lifecycle_keeps_request_pending_after_failed_task() {
        let now = std::time::Instant::now();
        let mut state = InterfaceState::default();
        state
            .stand_up_request
            .update_command(&MotionCommand::StandUp);

        let generation = mark_stand_up_started(&mut state, now).unwrap();

        assert!(state.stand_up_request.is_in_flight());

        apply_effect_task_result(
            &mut state,
            EffectTaskResult::StandUp {
                generation,
                success: false,
            },
        );

        assert!(!state.stand_up_request.is_in_flight());
        assert!(state.stand_up_request.is_pending());
        assert!(!state.stand_up_request.should_request(
            Some(booster_sdk::types::RobotMode::Prepare),
            now + Duration::from_millis(499),
            interface_parameters().stand_up_retry_interval,
            true,
        ));
    }

    #[test]
    fn stand_up_command_drop_keeps_in_flight_request_until_completion() {
        let retry_interval = Duration::from_millis(100);
        let now = std::time::Instant::now();
        let mut state = StandUpRequestState::default();

        state.update_command(&MotionCommand::StandUp);
        assert!(state.should_request(
            Some(booster_sdk::types::RobotMode::Prepare),
            now,
            retry_interval,
            true,
        ));

        let generation = state.record_attempt(now).unwrap();
        state.update_command(&MotionCommand::Prepare);

        assert!(state.is_in_flight());
        assert!(!state.should_request(
            Some(booster_sdk::types::RobotMode::Prepare),
            now + retry_interval,
            retry_interval,
            true,
        ));

        state.record_completion(generation, true);

        assert!(!state.is_pending());
        assert!(!state.is_in_flight());
    }

    #[test]
    fn stale_stand_up_completion_does_not_clear_newer_request() {
        let mut state = StandUpRequestState::Pending {
            generation: 2,
            last_attempt: None,
        };

        state.record_completion(1, true);

        assert!(state.is_pending());
        assert!(!state.is_in_flight());
    }

    #[test]
    fn stand_up_command_flap_does_not_start_overlapping_requests() {
        let retry_interval = Duration::from_millis(100);
        let now = std::time::Instant::now();
        let mut state = StandUpRequestState::default();

        state.update_command(&MotionCommand::StandUp);
        let generation = state.record_attempt(now).unwrap();
        state.update_command(&MotionCommand::Prepare);
        state.update_command(&MotionCommand::StandUp);

        assert!(state.is_in_flight());
        assert!(!state.should_request(
            Some(booster_sdk::types::RobotMode::Prepare),
            now + retry_interval,
            retry_interval,
            true,
        ));
        assert_eq!(state.record_attempt(now + retry_interval), None);

        state.record_completion(generation, false);

        assert!(state.is_pending());
        assert!(!state.is_in_flight());
    }

    #[test]
    fn visual_kick_completion_only_updates_matching_request() {
        let mut state = InterfaceState {
            visual_kick_in_flight: Some(VisualKickRequest::Start),
            ..Default::default()
        };

        apply_effect_task_result(
            &mut state,
            EffectTaskResult::VisualKick {
                request: VisualKickRequest::Stop,
                success: true,
            },
        );

        assert_eq!(state.visual_kick_in_flight, Some(VisualKickRequest::Start));
        assert!(!state.visual_kick.is_active());

        apply_effect_task_result(
            &mut state,
            EffectTaskResult::VisualKick {
                request: VisualKickRequest::Start,
                success: true,
            },
        );

        assert_eq!(state.visual_kick_in_flight, None);
        assert!(state.visual_kick.is_active());
    }

    #[test]
    fn visual_kick_request_maps_to_sdk_active_flag() {
        assert!(VisualKickRequest::Start.active());
        assert!(!VisualKickRequest::Stop.active());
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
            sdk_mode_for(control::DesiredMode::Walking),
            booster_sdk::types::RobotMode::Walking
        );
    }

    #[test]
    fn raw_robot_mode_ids_decode_known_modes() {
        assert_eq!(
            robot_mode_from_raw_id(booster_sdk::types::RobotMode::Walking as i32),
            Some(booster_sdk::types::RobotMode::Walking),
        );
    }

    #[test]
    fn raw_robot_mode_ids_reject_unknown_modes() {
        assert_eq!(robot_mode_from_raw_id(i32::MAX), None);
    }

    #[test]
    fn interface_detects_short_f1_or_stand_local_stop_requests() {
        assert!(!button_requests_local_stop_toggle(&Buttons {
            f1: None,
            stand: None,
            walking: None,
        }));
        assert!(button_requests_local_stop_toggle(&Buttons {
            f1: Some(ButtonPressType::Short),
            stand: None,
            walking: None,
        }));
        assert!(button_requests_local_stop_toggle(&Buttons {
            f1: None,
            stand: Some(ButtonPressType::Short),
            walking: None,
        }));
        assert!(!button_requests_local_stop_toggle(&Buttons {
            f1: Some(ButtonPressType::Long),
            stand: None,
            walking: None,
        }));
        assert!(!button_requests_local_stop_toggle(&Buttons {
            f1: None,
            stand: None,
            walking: Some(ButtonPressType::Short),
        }));
    }

    #[test]
    fn stand_up_retry_state_keeps_request_pending_until_success() {
        let retry_interval = Duration::from_millis(100);
        let now = std::time::Instant::now();
        let mut state = StandUpRequestState::default();

        state.update_command(&MotionCommand::StandUp);

        assert!(state.is_pending());
        assert!(!state.should_request(
            Some(booster_sdk::types::RobotMode::Walking),
            now,
            retry_interval,
            true,
        ));
        assert!(state.should_request(
            Some(booster_sdk::types::RobotMode::Prepare),
            now,
            retry_interval,
            true,
        ));

        let generation = state.record_attempt(now).unwrap();
        state.record_completion(generation, false);

        assert!(state.is_pending());
        assert!(!state.should_request(
            Some(booster_sdk::types::RobotMode::Prepare),
            now + Duration::from_millis(99),
            retry_interval,
            true,
        ));
        assert!(state.should_request(
            Some(booster_sdk::types::RobotMode::Damping),
            now + retry_interval,
            retry_interval,
            true,
        ));

        let generation = state.record_attempt(now + retry_interval).unwrap();
        state.record_completion(generation, true);
        state.update_command(&MotionCommand::StandUp);

        assert!(!state.is_pending());

        state.update_command(&MotionCommand::Prepare);
        state.update_command(&MotionCommand::StandUp);

        assert!(state.is_pending());
    }

    #[test]
    fn stand_up_retry_state_is_suppressed_during_emergency_damping() {
        let retry_interval = Duration::from_millis(100);
        let now = std::time::Instant::now();
        let mut state = StandUpRequestState::default();

        state.update_command(&MotionCommand::StandUp);

        assert!(!state.should_request(
            Some(booster_sdk::types::RobotMode::Damping),
            now,
            retry_interval,
            false,
        ));
        assert!(state.is_pending());
        assert!(state.should_request(
            Some(booster_sdk::types::RobotMode::Damping),
            now,
            retry_interval,
            true,
        ));
    }

    #[test]
    fn visual_kick_retry_state_does_not_change_before_success() {
        let mut state = control::VisualKickState::default();

        assert_eq!(
            visual_kick_transition_for(state, true),
            control::VisualKickTransition::Start
        );
        assert!(!state.is_active());

        state.update(true);
        assert_eq!(
            visual_kick_transition_for(state, true),
            control::VisualKickTransition::None
        );
        assert_eq!(
            visual_kick_transition_for(state, false),
            control::VisualKickTransition::Stop
        );
        assert!(state.is_active());

        state.update(false);
        assert_eq!(
            visual_kick_transition_for(state, false),
            control::VisualKickTransition::None
        );
    }

    #[test]
    fn visual_kick_transition_retry_waits_for_retry_interval() {
        let retry_interval = Duration::from_millis(100);
        let now = std::time::Instant::now();

        assert!(visual_kick_retry_due(None, now, retry_interval));
        assert!(!visual_kick_retry_due(
            Some(now),
            now + Duration::from_millis(99),
            retry_interval,
        ));
        assert!(visual_kick_retry_due(
            Some(now),
            now + retry_interval,
            retry_interval,
        ));
    }

    #[tokio::test]
    async fn sdk_call_returns_none_on_timeout() {
        let result = await_sdk_call(
            std::future::pending::<std::result::Result<(), std::convert::Infallible>>(),
            Duration::from_millis(1),
            "pending test operation",
        )
        .await;

        assert!(result.is_none());
    }
}
