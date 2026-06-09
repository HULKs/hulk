use std::{boxed::Box, future::Future, pin::Pin, sync::Arc};

use booster_sdk::client::{BoosterClient, light_control::LightControlClient};
use color_eyre::{Result, eyre::WrapErr};
use kinematics::joints::head::HeadJoints;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    buttons::{ButtonPressType, Buttons},
    motion_command::MotionCommand,
};

mod control;
mod kick_transport;

#[derive(Serialize, Deserialize, Message)]
pub enum LedCommand {
    SetParam { r: u8, g: u8, b: u8 },
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub walking: types::parameters::RLWalkingParameters,
    pub move_robot_message_interval: std::time::Duration,
    pub kicking: types::parameters::BoosterKickingParameters,
    pub rotate_head_message_interval: std::time::Duration,
    pub mode_poll_interval: std::time::Duration,
    pub mode_retry_interval: std::time::Duration,
    pub remote_stop_toggle: bool,
}

struct InterfaceState {
    confirmed_mode: Option<booster_sdk::types::RobotMode>,
    desired_mode: Option<control::DesiredMode>,
    last_mode_request: std::time::Instant,
    last_mode_poll: std::time::Instant,
    last_move_robot: std::time::Instant,
    last_rotate_head: std::time::Instant,
    last_kick: std::time::Instant,
    last_visual_kick_attempt: Option<std::time::Instant>,
    last_motion_command: MotionCommand,
    visual_kick: control::VisualKickState,
    stand_up_request: StandUpRequestState,
    local_stop_toggle: bool,
}

impl Default for InterfaceState {
    fn default() -> Self {
        let now = std::time::Instant::now();
        Self {
            confirmed_mode: None,
            desired_mode: None,
            last_mode_request: now,
            last_mode_poll: now,
            last_move_robot: now,
            last_rotate_head: now,
            last_kick: now,
            last_visual_kick_attempt: None,
            last_motion_command: MotionCommand::Damping,
            visual_kick: control::VisualKickState::default(),
            stand_up_request: StandUpRequestState::default(),
            local_stop_toggle: false,
        }
    }
}

#[derive(Debug, Default)]
struct StandUpRequestState {
    was_stand_up: bool,
    pending: bool,
    last_attempt: Option<std::time::Instant>,
}

impl StandUpRequestState {
    fn update_command(&mut self, command: &MotionCommand) {
        let is_stand_up = matches!(command, MotionCommand::StandUp);
        if is_stand_up && !self.was_stand_up {
            self.pending = true;
            self.last_attempt = None;
        } else if !is_stand_up {
            self.pending = false;
            self.last_attempt = None;
        }
        self.was_stand_up = is_stand_up;
    }

    #[cfg(test)]
    fn is_pending(&self) -> bool {
        self.pending
    }

    fn should_request(
        &self,
        confirmed_mode: Option<booster_sdk::types::RobotMode>,
        now: std::time::Instant,
        retry_interval: std::time::Duration,
        allow_stand_up: bool,
    ) -> bool {
        self.pending
            && allow_stand_up
            && matches!(
                confirmed_mode,
                Some(
                    booster_sdk::types::RobotMode::Prepare | booster_sdk::types::RobotMode::Damping
                )
            )
            && self.last_attempt.is_none_or(|last_attempt| {
                now.saturating_duration_since(last_attempt) >= retry_interval
            })
    }

    fn record_attempt(&mut self, now: std::time::Instant) {
        self.last_attempt = Some(now);
    }

    fn record_success(&mut self) {
        self.pending = false;
        self.last_attempt = None;
    }
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = Arc::new(
        ctx.create_node("booster_interface")
            .build()
            .await
            .wrap_err("failed to create booster_interface node")?,
    );
    let parameters = node
        .bind_parameter_as::<Parameters>("booster_interface")
        .wrap_err("failed to bind booster_interface parameters")?;
    let booster_client = Arc::new(BoosterClient::new().wrap_err("failed to create BoosterClient")?);
    let light_control_client =
        Arc::new(LightControlClient::new().wrap_err("failed to create LightControlClient")?);
    let kick_ball_publisher = kick_transport::KickBallPublisher::new(ctx.session())
        .await
        .wrap_err("failed to create kick ball publisher")?;

    let motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")
        .wrap_err("failed to create motion_command subscriber")?
        .build()
        .await
        .wrap_err("failed to build motion_command subscriber")?;
    let head_joints_sub = node
        .subscriber::<HeadJoints<f32>>("head_joints_command")
        .wrap_err("failed to create head_joints_command subscriber")?
        .build()
        .await
        .wrap_err("failed to build head_joints_command subscriber")?;
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

    let mut state = InterfaceState::default();
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(10));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut latest_head_joints: Option<HeadJoints<f32>> = None;

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        tokio::select! {
            motion_command = motion_command_sub.recv() => {
                state.last_motion_command = motion_command?;
            }
            head_joints = head_joints_sub.recv() => {
                latest_head_joints = Some(head_joints?);
            }
            led_command = led_command_sub.recv() => {
                let light_control_client = light_control_client.clone();
                tokio::spawn(handle_led_command(light_control_client, led_command?));
            }
            buttons = buttons_sub.recv() => {
                let buttons = buttons?;
                if button_requests_local_stop_toggle(&buttons) {
                    state.local_stop_toggle = !state.local_stop_toggle;
                }
            }
            _ = tick.tick() => {
                let emergency_damping = state.local_stop_toggle != parameters.remote_stop_toggle;
                drive_booster_effects(
                    &mut state,
                    &booster_client,
                    &kick_ball_publisher,
                    latest_head_joints,
                    emergency_damping,
                    parameters,
                ).await;
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
        control::DesiredMode::Custom => booster_sdk::types::RobotMode::Custom,
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

async fn poll_mode(client: &BoosterClient) -> Option<booster_sdk::types::RobotMode> {
    match client.get_mode().await {
        Ok(mode) => mode.mode_enum(),
        Err(error) => {
            log::error!("failed to poll booster mode: {error}");
            None
        }
    }
}

async fn request_mode(client: &BoosterClient, desired_mode: control::DesiredMode) {
    let mode = sdk_mode_for(desired_mode);
    if let Err(error) = client.change_mode(mode).await {
        log::error!("failed to request booster mode {mode:?}: {error}");
    }
}

async fn drive_booster_effects(
    state: &mut InterfaceState,
    booster_client: &BoosterClient,
    kick_ball_publisher: &kick_transport::KickBallPublisher,
    latest_head_joints: Option<HeadJoints<f32>>,
    emergency_damping: bool,
    parameters: &Parameters,
) {
    let mut now = std::time::Instant::now();

    if now.duration_since(state.last_mode_poll) >= parameters.mode_poll_interval {
        state.confirmed_mode = poll_mode(booster_client).await;
        now = std::time::Instant::now();
        state.last_mode_poll = now;
    }

    let desired_mode = control::desired_mode_for(&state.last_motion_command, emergency_damping);
    let confirmed_desired_mode = state.confirmed_mode == Some(sdk_mode_for(desired_mode));
    if !confirmed_desired_mode
        && (state.desired_mode != Some(desired_mode)
            || now.duration_since(state.last_mode_request) >= parameters.mode_retry_interval)
    {
        request_mode(booster_client, desired_mode).await;
        now = std::time::Instant::now();
        state.desired_mode = Some(desired_mode);
        state.last_mode_request = now;
    }

    let walking_allowed =
        control::confirmed_mode_allows_walking(state.confirmed_mode) && !emergency_damping;

    state
        .stand_up_request
        .update_command(&state.last_motion_command);
    if state.stand_up_request.should_request(
        state.confirmed_mode,
        now,
        parameters.mode_retry_interval,
        !emergency_damping,
    ) {
        match booster_client.get_up().await {
            Ok(()) => {
                now = std::time::Instant::now();
                state.stand_up_request.record_success();
            }
            Err(error) => {
                log::error!("failed to request get_up: {error}");
                now = std::time::Instant::now();
                state.stand_up_request.record_attempt(now);
            }
        }
    }

    if !walking_allowed {
        let transition = visual_kick_transition_for(state.visual_kick, false);
        if transition == control::VisualKickTransition::Stop
            && visual_kick_retry_due(
                state.last_visual_kick_attempt,
                now,
                parameters.mode_retry_interval,
            )
        {
            match booster_client.visual_kick(false).await {
                Ok(()) => {
                    state.visual_kick.update(false);
                    state.last_visual_kick_attempt = None;
                }
                Err(error) => {
                    log::error!("failed to stop visual kick: {error}");
                    now = std::time::Instant::now();
                    state.last_visual_kick_attempt = Some(now);
                }
            }
        } else if transition == control::VisualKickTransition::None {
            state.last_visual_kick_attempt = None;
        }
        return;
    }

    if now.duration_since(state.last_move_robot) >= parameters.move_robot_message_interval {
        let step =
            control::step_from_motion_command(&state.last_motion_command, &parameters.walking);
        if let Err(error) = booster_client
            .move_robot(step.forward, step.left, step.turn)
            .await
        {
            log::error!("failed to send move_robot: {error}");
        }
        now = std::time::Instant::now();
        state.last_move_robot = now;
    }

    if now.duration_since(state.last_rotate_head) >= parameters.rotate_head_message_interval
        && let Some(head_joints) = latest_head_joints
    {
        if let Err(error) = booster_client
            .rotate_head(head_joints.pitch, head_joints.yaw)
            .await
        {
            log::error!("failed to rotate head: {error}");
        }
        now = std::time::Instant::now();
        state.last_rotate_head = now;
    }

    let should_visual_kick = matches!(state.last_motion_command, MotionCommand::VisualKick { .. });
    match visual_kick_transition_for(state.visual_kick, should_visual_kick) {
        control::VisualKickTransition::Start
            if visual_kick_retry_due(
                state.last_visual_kick_attempt,
                now,
                parameters.mode_retry_interval,
            ) =>
        {
            match booster_client.visual_kick(true).await {
                Ok(()) => {
                    now = std::time::Instant::now();
                    state.visual_kick.update(true);
                    state.last_visual_kick_attempt = None;
                }
                Err(error) => {
                    log::error!("failed to start visual kick: {error}");
                    now = std::time::Instant::now();
                    state.last_visual_kick_attempt = Some(now);
                }
            }
        }
        control::VisualKickTransition::Stop
            if visual_kick_retry_due(
                state.last_visual_kick_attempt,
                now,
                parameters.mode_retry_interval,
            ) =>
        {
            match booster_client.visual_kick(false).await {
                Ok(()) => {
                    now = std::time::Instant::now();
                    state.visual_kick.update(false);
                    state.last_visual_kick_attempt = None;
                }
                Err(error) => {
                    log::error!("failed to stop visual kick: {error}");
                    now = std::time::Instant::now();
                    state.last_visual_kick_attempt = Some(now);
                }
            }
        }
        control::VisualKickTransition::Start | control::VisualKickTransition::Stop => {}
        control::VisualKickTransition::None => {
            state.last_visual_kick_attempt = None;
        }
    }

    if should_visual_kick
        && now.duration_since(state.last_kick) >= parameters.kicking.kick_message_interval
        && let Some(kick) = control::kick_from_motion_command(
            &state.last_motion_command,
            std::time::SystemTime::now(),
            &parameters.kicking,
        )
    {
        if let Err(error) = kick_ball_publisher.publish(&kick).await {
            log::error!("failed to publish visual kick command: {error}");
        }
        state.last_kick = std::time::Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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

        state.record_attempt(now);

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

        state.record_success();
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
}
