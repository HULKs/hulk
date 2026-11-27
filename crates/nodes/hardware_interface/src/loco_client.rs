use std::{sync::Arc, time::Duration};

use booster::RobotMode;
use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use serde::Deserialize;

use crate::rpc_transport::ZenohRpcClient;

const LOCO_API_TOPIC: &str = "rt/LocoApiTopic";
const CHANGE_MODE_API_ID: i32 = 2000;
const MOVE_API_ID: i32 = 2001;
const ROTATE_HEAD_API_ID: i32 = 2004;
const GET_UP_API_ID: i32 = 2008;
const GET_MODE_API_ID: i32 = 2017;
const VISUAL_KICK_API_ID: i32 = 2038;
const VISUAL_KICK_V2: i32 = 1;

#[derive(Clone)]
pub struct LocoClient {
    rpc: Arc<ZenohRpcClient>,
}

impl LocoClient {
    pub async fn new(session: &zenoh::Session) -> Result<Self> {
        Ok(Self {
            rpc: Arc::new(ZenohRpcClient::new(session, LOCO_API_TOPIC).await?),
        })
    }

    pub async fn change_mode(&self, mode: RobotMode, timeout: Duration) -> Result<()> {
        self.rpc
            .call(CHANGE_MODE_API_ID, change_mode_body(mode), timeout)
            .await?;
        Ok(())
    }

    pub async fn get_mode(&self, timeout: Duration) -> Result<RobotMode> {
        let body = self.rpc.call(GET_MODE_API_ID, "", timeout).await?;
        parse_get_mode_body(&body)
    }

    pub async fn move_robot(&self, vx: f32, vy: f32, vyaw: f32, timeout: Duration) -> Result<()> {
        self.rpc
            .call(MOVE_API_ID, move_robot_body(vx, vy, vyaw), timeout)
            .await?;
        Ok(())
    }

    pub async fn rotate_head(&self, pitch: f32, yaw: f32, timeout: Duration) -> Result<()> {
        self.rpc
            .call(ROTATE_HEAD_API_ID, rotate_head_body(pitch, yaw), timeout)
            .await?;
        Ok(())
    }

    pub async fn get_up(&self, timeout: Duration) -> Result<()> {
        self.rpc.call(GET_UP_API_ID, "", timeout).await?;
        Ok(())
    }

    pub async fn visual_kick(&self, start: bool, timeout: Duration) -> Result<()> {
        self.rpc
            .call(VISUAL_KICK_API_ID, visual_kick_body(start), timeout)
            .await?;
        Ok(())
    }
}

fn change_mode_body(mode: RobotMode) -> String {
    serde_json::json!({ "mode": i32::from(mode) }).to_string()
}

fn parse_get_mode_body(body: &str) -> Result<RobotMode> {
    #[derive(Deserialize)]
    struct GetModeResponse {
        mode: i32,
    }

    let GetModeResponse { mode } =
        serde_json::from_str(body).wrap_err("invalid get-mode response")?;
    let mode =
        RobotMode::try_from(mode).map_err(|error| eyre!("get-mode response: {error} ({mode})"))?;
    if mode == RobotMode::Unknown {
        bail!("get-mode response reports an unknown robot mode");
    }
    Ok(mode)
}

fn move_robot_body(vx: f32, vy: f32, vyaw: f32) -> String {
    format!(r#"{{"vx":{vx},"vy":{vy},"vyaw":{vyaw}}}"#)
}

fn rotate_head_body(pitch: f32, yaw: f32) -> String {
    format!(r#"{{"pitch":{pitch},"yaw":{yaw}}}"#)
}

fn visual_kick_body(start: bool) -> String {
    serde_json::json!({ "start": start, "version": VISUAL_KICK_V2 }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_mode_parses_known_modes() {
        for (body, expected) in [
            (r#"{"mode":0}"#, RobotMode::Damping),
            (r#"{"mode":1}"#, RobotMode::Prepare),
            (r#"{"mode":2}"#, RobotMode::Walking),
            (r#"{"mode":3}"#, RobotMode::Custom),
            (r#" {"mode":4}"#, RobotMode::Soccer),
        ] {
            assert_eq!(parse_get_mode_body(body).unwrap(), expected);
        }
    }

    #[test]
    fn get_mode_rejects_malformed_and_unsupported_response_bodies() {
        for body in [
            "",
            "{",
            "{}",
            r#"{"mode":"4"}"#,
            r#"{"mode":2147483648}"#,
            r#"{"mode":-1}"#,
            r#"{"mode":5}"#,
        ] {
            assert!(parse_get_mode_body(body).is_err(), "accepted {body:?}");
        }
    }

    #[test]
    fn change_mode_body_uses_robot_mode_integer() {
        assert_eq!(change_mode_body(RobotMode::Soccer), r#"{"mode":4}"#);
    }

    #[test]
    fn move_robot_body_uses_booster_velocity_fields() {
        assert_eq!(
            move_robot_body(0.1, -0.2, 0.3),
            r#"{"vx":0.1,"vy":-0.2,"vyaw":0.3}"#
        );
    }

    #[test]
    fn rotate_head_body_uses_pitch_and_yaw() {
        assert_eq!(rotate_head_body(0.4, -0.5), r#"{"pitch":0.4,"yaw":-0.5}"#);
    }

    #[test]
    fn visual_kick_body_uses_v2_version() {
        assert_eq!(visual_kick_body(true), r#"{"start":true,"version":1}"#);
    }
}
