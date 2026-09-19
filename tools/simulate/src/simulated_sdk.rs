//! Acknowledge supported mode changes so the real actuator can enter Custom mode.
use booster::{RpcReqMsg, RpcRespMsg};
use color_eyre::{Result, eyre::eyre};
use ros_z::prelude::Context;
use std::sync::Arc;

pub async fn run(context: Arc<Context>) -> Result<()> {
    let requests = context
        .session()
        .declare_subscriber("rt/LocoApiTopicReq")
        .await
        .map_err(|e| eyre!("{e}"))?;
    while let Ok(sample) = requests.recv_async().await {
        let request: RpcReqMsg = cdr::deserialize(&sample.payload().to_bytes())?;
        let header: serde_json::Value = serde_json::from_str(&request.header)?;
        let body: serde_json::Value = serde_json::from_str(&request.body).unwrap_or_default();
        let supported = header["api_id"].as_i64() == Some(2000)
            && matches!(body["mode"].as_i64(), Some(0 | 1 | 3));
        // Prepare/Damping have no SDK pose controller here; the upstream actuator
        // still supplies damping and Custom joint targets through rt/joint_ctrl.
        let response = RpcRespMsg {
            uuid: request.uuid,
            header: serde_json::json!({"status": if supported { 0 } else { 1 }}).to_string(),
            body: if supported {
                String::new()
            } else {
                "Unsupported simulator SDK request".into()
            },
        };
        let bytes = cdr::serialize::<_, _, cdr::CdrLe>(&response, cdr::Infinite)?;
        context
            .session()
            .put("rt/LocoApiTopicResp", bytes)
            .await
            .map_err(|e| eyre!("{e}"))?;
    }
    Ok(())
}
