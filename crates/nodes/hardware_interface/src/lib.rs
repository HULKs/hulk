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

use color_eyre::{Result, eyre::WrapErr};
use serde::{Deserialize, Serialize};
use tokio::{sync::watch, task::JoinSet};
use tracing::{error, info};

use booster::{LedColor, LowCommand, RobotMode};
use retry_worker::{RetryCommand, run_retrying_rpc_worker};
use ros_z::{parameter::NodeParameters, prelude::*};
use types::robot_command::{JointsCommand, MotionCommand, MotionType};

mod joint_control;
mod light_client;
mod loco_client;
mod retry_worker;
mod rpc_transport;

pub use light_client::LightClient;
pub use rpc_transport::ZenohRpcClient;

use crate::joint_control::JointControlPublisher;

const MOTION_COMMAND_TOPIC: &str = "commands/motion_command";

#[derive(Debug, Serialize, Deserialize, Message)]
pub enum LedCommand {
    SetParam { r: u8, g: u8, b: u8 },
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub joint_control_message_interval: std::time::Duration,
    pub rotate_head_message_interval: std::time::Duration,
    pub sdk_request_timeout: std::time::Duration,
}

#[derive(Debug, Clone, Copy)]
enum RpcActionKind {
    ChangeMode,
    LedControl,
}

impl RpcActionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::ChangeMode => "change_mode",
            Self::LedControl => "led_control",
        }
    }
}

#[derive(Default)]
struct RpcDiagnostics {
    next_sequence: AtomicU64,
    change_mode_in_flight: AtomicUsize,
    led_control_in_flight: AtomicUsize,
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
            RpcActionKind::LedControl => &self.led_control_in_flight,
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
            target: "hardware_interface::rpc",
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DesiredLed {
    Set(LedColor),
    Stop,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = Arc::new(
        ctx.create_node("hardware_interface")
            .build()
            .await
            .wrap_err("failed to create hardware_interface node")?,
    );
    let parameters = Arc::new(
        node.bind_parameter_as::<Parameters>("hardware_interface")
            .wrap_err("failed to bind hardware_interface parameters")?,
    );

    let rpc_diagnostics = Arc::new(RpcDiagnostics::default());

    let mut join_set = JoinSet::new();

    join_set.spawn(joints_command_worker(
        ctx.clone(),
        node.clone(),
        parameters.clone(),
        rpc_diagnostics.clone(),
    ));
    join_set.spawn(led_command_worker(
        ctx,
        node,
        parameters,
        rpc_diagnostics.clone(),
    ));

    join_set.join_all().await;

    Ok(())
}

async fn joints_command_worker(
    ctx: Arc<Context>,
    node: Arc<Node>,
    parameters: Arc<NodeParameters<Parameters>>,
    rpc_diagnostics: Arc<RpcDiagnostics>,
) -> Result<()> {
    let motion_command_sub = node
        .subscriber::<MotionCommand>(MOTION_COMMAND_TOPIC)
        .build()
        .await
        .wrap_err("failed to build motion_command cache")?;
    let joint_control_publisher = JointControlPublisher::new(ctx.session())
        .await
        .wrap_err("failed to create joint control publisher")?;
    let loco_client = Arc::new(
        loco_client::LocoClient::new(ctx.session())
            .await
            .wrap_err("failed to create LocoClient")?,
    );
    let mode_command_sender = spawn_mode_worker(loco_client.clone(), rpc_diagnostics.clone());

    let mut assumed_robot_mode = RobotMode::Damping;

    loop {
        let MotionCommand {
            motion_type,
            joints_command,
        } = motion_command_sub.recv().await?;

        let timeout = parameters.snapshot().typed().sdk_request_timeout;

        let low_command = low_command_from_joints_command(joints_command);

        let robot_mode = booster_mode_from_motion_type(motion_type);

        if assumed_robot_mode != robot_mode {
            send_retry_command(&mode_command_sender, robot_mode, timeout, "change_mode");

            assumed_robot_mode = robot_mode;
        }

        joint_control_publisher.publish(&low_command).await?;
    }
}

fn low_command_from_joints_command(joints_command: JointsCommand) -> LowCommand {
    LowCommand {
        command_type: booster::CommandType::Serial,
        motor_commands: joints_command
            .into_iter()
            .map(|motor_command| booster::MotorCommand {
                position: motor_command.position,
                velocity: motor_command.velocity,
                torque: motor_command.torque,
                kp: motor_command.kp,
                kd: motor_command.kd,

                command_type: booster::CommandType::Serial,
                weight: 1.0,
            })
            .collect(),
    }
}

fn booster_mode_from_motion_type(motion_type: MotionType) -> RobotMode {
    match motion_type {
        MotionType::Damping => RobotMode::Damping,
        MotionType::Stand => RobotMode::Prepare,
        _ => RobotMode::Custom,
    }
}

async fn led_command_worker(
    ctx: Arc<Context>,
    node: Arc<Node>,
    parameters: Arc<NodeParameters<Parameters>>,
    rpc_diagnostics: Arc<RpcDiagnostics>,
) -> Result<()> {
    let led_command_sub = node
        .subscriber::<LedCommand>("commands/led_command")
        .build()
        .await
        .wrap_err("failed to build commands/led_command subscriber")?;
    let light_control_client = Arc::new(
        light_client::LightClient::new(ctx.session())
            .await
            .wrap_err("failed to create LightClient")?,
    );

    let led_command_sender = spawn_led_worker(light_control_client, rpc_diagnostics.clone());

    loop {
        let led_command = led_command_sub.recv().await?;
        let timeout = parameters.snapshot().typed().sdk_request_timeout;

        let desired_led = desired_led_for(led_command);

        send_retry_command(&led_command_sender, desired_led, timeout, "led_control");
    }
}

fn desired_led_for(led_command: LedCommand) -> DesiredLed {
    match led_command {
        LedCommand::SetParam { r, g, b } => DesiredLed::Set(LedColor { r, g, b }),
        LedCommand::Stop => DesiredLed::Stop,
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
                target: "hardware_interface::rpc",
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
                target: "hardware_interface::rpc",
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
        error!(target: "hardware_interface::rpc", operation, "failed to send rpc worker command");
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
                error!(target: "hardware_interface::rpc", operation = %operation, error = %error, "booster rpc timed out");
            } else {
                error!(target: "hardware_interface::rpc", operation = %operation, error = %error, "booster rpc failed");
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
    fn rpc_diagnostics_assigns_sequences_and_action_local_in_flight_counts() {
        let diagnostics = std::sync::Arc::new(RpcDiagnostics::default());

        let first_change_mode = diagnostics.begin(RpcActionKind::ChangeMode);
        let second_change_mode = diagnostics.begin(RpcActionKind::ChangeMode);
        let first_led_control = diagnostics.begin(RpcActionKind::LedControl);

        assert_eq!(first_change_mode.sequence, 1);
        assert_eq!(first_change_mode.in_flight_at_start, 1);
        assert_eq!(second_change_mode.sequence, 2);
        assert_eq!(second_change_mode.in_flight_at_start, 2);
        assert_eq!(first_led_control.sequence, 3);
        assert_eq!(first_led_control.in_flight_at_start, 1);
    }

    #[test]
    fn rpc_attempt_finish_decrements_action_local_in_flight_count() {
        let diagnostics = std::sync::Arc::new(RpcDiagnostics::default());
        let attempt = diagnostics.begin(RpcActionKind::LedControl);

        assert_eq!(
            diagnostics
                .led_control_in_flight
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        attempt.finish("ok");

        assert_eq!(
            diagnostics
                .led_control_in_flight
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[tokio::test]
    async fn rpc_call_waits_for_future_owned_timeout() {
        let diagnostics = std::sync::Arc::new(RpcDiagnostics::default());
        let attempt = diagnostics.begin(RpcActionKind::LedControl);
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
