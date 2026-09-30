use std::{sync::Arc, time::Duration};

use booster::LedColor;
use color_eyre::Result;

use crate::rpc_transport::ZenohRpcClient;

const LIGHT_CONTROL_API_TOPIC: &str = "rt/LightControlApiTopic";
const SET_LED_LIGHT_COLOR_API_ID: i32 = 2000;
const STOP_LED_LIGHT_CONTROL_API_ID: i32 = 2001;

#[derive(Clone)]
pub struct LightClient {
    rpc: Arc<ZenohRpcClient>,
}

impl LightClient {
    pub async fn new(session: &zenoh::Session) -> Result<Self> {
        Ok(Self {
            rpc: Arc::new(ZenohRpcClient::new(session, LIGHT_CONTROL_API_TOPIC).await?),
        })
    }

    pub async fn set_led_light_color(&self, color: LedColor, timeout: Duration) -> Result<()> {
        self.rpc
            .call(SET_LED_LIGHT_COLOR_API_ID, led_color_body(color), timeout)
            .await
    }

    pub async fn stop_led_light_control(&self, timeout: Duration) -> Result<()> {
        self.rpc
            .call(STOP_LED_LIGHT_CONTROL_API_ID, "", timeout)
            .await
    }
}

fn led_color_body(color: LedColor) -> String {
    format!(r#"{{"r":{},"g":{},"b":{}}}"#, color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use booster::LedColor;

    #[test]
    fn led_color_body_uses_rgb_fields() {
        let body = led_color_body(LedColor { r: 1, g: 2, b: 3 });

        assert_eq!(body, r#"{"r":1,"g":2,"b":3}"#);
    }
}
