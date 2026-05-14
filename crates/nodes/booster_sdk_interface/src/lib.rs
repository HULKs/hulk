use std::sync::Arc;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster_sdk::{
    client::{BoosterClient, light_control::LightControlClient},
    types::RobotMode as RobotModeSdk,
};
use ros_z::{
    IntoEyreResultExt, Message, Service, ServiceTypeInfo, TypeInfo, context::Context,
    dynamic::SchemaError, service::ServiceServer,
};

#[derive(Serialize, Deserialize, Message)]
pub enum LedCommand {
    SetParam { r: u8, g: u8, b: u8 },
    Stop,
}

#[derive(Serialize, Deserialize, Message)]
pub enum HighLevelCommand {
    ChangeMode { mode: RobotMode },
    MoveRobot { forward: f32, left: f32, turn: f32 },
    RotateHead { pitch: f32, yaw: f32 },
    RotateHeadWithDirection { pitch: i32, yaw: i32 },
    LieDown,
    GetUp,
    GetUpWithMode { mode: RobotMode },
    EnterWbcGait,
    ExitWbcGait,
    VisualKick { start: bool },
    ResetOdometer,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub enum RobotMode {
    /// Unknown mode, typically used for error handling.
    Unknown = -1,

    /// Damping mode, motors are compliant
    Damping = 0,

    /// Prepare mode, standing pose
    Prepare = 1,

    /// Walking mode, active locomotion
    Walking = 2,

    /// Custom mode, user-defined behavior
    Custom = 3,

    /// Soccer mode
    Soccer = 4,
}

impl From<RobotModeSdk> for RobotMode {
    fn from(sdk_mode: RobotModeSdk) -> Self {
        match sdk_mode {
            RobotModeSdk::Unknown => Self::Unknown,
            RobotModeSdk::Damping => Self::Damping,
            RobotModeSdk::Prepare => Self::Prepare,
            RobotModeSdk::Walking => Self::Walking,
            RobotModeSdk::Custom => Self::Custom,
            RobotModeSdk::Soccer => Self::Soccer,
            _ => Self::Unknown,
        }
    }
}

impl From<Option<RobotModeSdk>> for RobotMode {
    fn from(sdk_mode: Option<RobotModeSdk>) -> Self {
        match sdk_mode {
            Some(RobotModeSdk::Unknown) => Self::Unknown,
            Some(RobotModeSdk::Damping) => Self::Damping,
            Some(RobotModeSdk::Prepare) => Self::Prepare,
            Some(RobotModeSdk::Walking) => Self::Walking,
            Some(RobotModeSdk::Custom) => Self::Custom,
            Some(RobotModeSdk::Soccer) => Self::Soccer,
            _ => Self::Unknown,
        }
    }
}

impl From<RobotMode> for RobotModeSdk {
    fn from(mode: RobotMode) -> Self {
        match mode {
            RobotMode::Unknown => Self::Unknown,
            RobotMode::Damping => Self::Damping,
            RobotMode::Prepare => Self::Prepare,
            RobotMode::Walking => Self::Walking,
            RobotMode::Custom => Self::Custom,
            RobotMode::Soccer => Self::Soccer,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct GetRobotModeRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct GetRobotModeResponse {
    robot_mode: RobotMode,
}

pub struct GetRobotMode;

impl ServiceTypeInfo for GetRobotMode {
    fn service_type_info() -> std::prelude::v1::Result<ros_z::prelude::TypeInfo, SchemaError> {
        Ok(TypeInfo::new("hardware_interface::GetRobotMode", None))
    }
}

impl Service for GetRobotMode {
    type Request = GetRobotModeRequest;

    type Response = GetRobotModeResponse;
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = Arc::new(
        ctx.create_node("booster_sdk_interface")
            .build()
            .await
            .into_eyre()?,
    );

    let high_level_interface_client = Arc::new(BoosterClient::new()?);
    let light_control_client = Arc::new(LightControlClient::new()?);

    let led_command_sub = node
        .subscriber::<LedCommand>("commands/led_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let high_level_command_sub = node
        .subscriber::<HighLevelCommand>("commands/high_level_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    let mut robot_mode_service: ServiceServer<GetRobotMode> = node
        .create_service_server::<GetRobotMode>("services/get_robot_mode")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    loop {
        tokio::select! {
            led_command = led_command_sub.recv() => {
                let led_command = led_command.into_eyre()?;

                tokio::spawn({
                    let light_control_client = light_control_client.clone();

                    handle_led_command(light_control_client, led_command)
                });
            },
            high_level_command = high_level_command_sub.recv() => {
                let high_level_command = high_level_command.into_eyre()?;

                tokio::spawn({
                    let high_level_interface_client = high_level_interface_client.clone();

                    handle_high_level_command(high_level_interface_client, high_level_command)
                });
            },
            robot_mode_request = robot_mode_service.take_request_async() => {
                let robot_mode_request = robot_mode_request.into_eyre()?;

                let client = high_level_interface_client.clone();
                tokio::spawn(async move {
                    handle_robot_mode_request(client, robot_mode_request).await;
                });
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

async fn handle_high_level_command(
    high_level_interface_client: Arc<BoosterClient>,
    high_level_command: HighLevelCommand,
) -> Result<()> {
    match high_level_command {
        HighLevelCommand::ChangeMode { mode } => high_level_interface_client
            .change_mode(mode.into())
            .await
            .into_eyre(),
        HighLevelCommand::MoveRobot {
            forward,
            left,
            turn,
        } => high_level_interface_client
            .move_robot(forward, left, turn)
            .await
            .into_eyre(),
        HighLevelCommand::RotateHead { pitch, yaw } => high_level_interface_client
            .rotate_head(pitch, yaw)
            .await
            .into_eyre(),
        HighLevelCommand::RotateHeadWithDirection { pitch, yaw } => high_level_interface_client
            .rotate_head_with_direction(pitch, yaw)
            .await
            .into_eyre(),
        HighLevelCommand::LieDown => high_level_interface_client.lie_down().await.into_eyre(),
        HighLevelCommand::GetUp => high_level_interface_client.get_up().await.into_eyre(),
        HighLevelCommand::GetUpWithMode { mode } => high_level_interface_client
            .get_up_with_mode(mode.into())
            .await
            .into_eyre(),
        HighLevelCommand::EnterWbcGait => high_level_interface_client
            .enter_wbc_gait()
            .await
            .into_eyre(),
        HighLevelCommand::ExitWbcGait => high_level_interface_client
            .exit_wbc_gait()
            .await
            .into_eyre(),
        HighLevelCommand::VisualKick { start } => high_level_interface_client
            .visual_kick(start)
            .await
            .into_eyre(),
        HighLevelCommand::ResetOdometer => high_level_interface_client
            .reset_odometry()
            .await
            .into_eyre(),
    }
}

async fn handle_robot_mode_request(
    high_level_interface_client: Arc<BoosterClient>,
    robot_mode_request: ros_z::service::ServiceRequest<GetRobotMode>,
) {
    match high_level_interface_client.get_mode().await {
        Ok(mode) => {
            let robot_mode: RobotMode = mode.mode_enum().into();
            let get_robot_mode_response = GetRobotModeResponse { robot_mode };
            if let Err(e) = robot_mode_request
                .reply_async(&get_robot_mode_response)
                .await
            {
                log::error!("failed to reply to robot mode request: {e}");
            }
        }
        Err(e) => {
            log::error!("failed to get robot mode from booster client: {e}");
            let get_robot_mode_response = GetRobotModeResponse {
                robot_mode: RobotMode::Unknown,
            };
            if let Err(e) = robot_mode_request
                .reply_async(&get_robot_mode_response)
                .await
            {
                log::error!("failed to reply to robot mode request after error: {e}");
            }
        }
    }
}
