use anyhow::{anyhow, bail, Context};
use serde_json::Value;

#[derive(Debug)]
pub enum SupportFoot {
    Left,
    Right,
}

impl TryFrom<&Value> for SupportFoot {
    type Error = anyhow::Error;

    fn try_from(replay_frame: &Value) -> anyhow::Result<Self> {
        let fsr_left = replay_frame
            .get("fsrLeft")
            .ok_or_else(|| anyhow!("replay_frame.get(\"fsrLeft\")"))?;
        let fsr_left_front_left =
            extract_value_from_fsr(fsr_left, "frontLeft").context("fsr_left")?;
        let fsr_left_front_right =
            extract_value_from_fsr(fsr_left, "frontRight").context("fsr_left")?;
        let fsr_left_rear_left =
            extract_value_from_fsr(fsr_left, "rearLeft").context("fsr_left")?;
        let fsr_left_rear_right =
            extract_value_from_fsr(fsr_left, "rearRight").context("fsr_left")?;

        let fsr_right = replay_frame
            .get("fsrRight")
            .ok_or_else(|| anyhow!("replay_frame.get(\"fsrRight\")"))?;
        let fsr_right_front_left =
            extract_value_from_fsr(fsr_right, "frontLeft").context("fsr_right")?;
        let fsr_right_front_right =
            extract_value_from_fsr(fsr_right, "frontRight").context("fsr_right")?;
        let fsr_right_rear_left =
            extract_value_from_fsr(fsr_right, "rearLeft").context("fsr_right")?;
        let fsr_right_rear_right =
            extract_value_from_fsr(fsr_right, "rearRight").context("fsr_right")?;

        let left_sum =
            fsr_left_front_left + fsr_left_front_right + fsr_left_rear_left + fsr_left_rear_right;
        let right_sum = fsr_right_front_left
            + fsr_right_front_right
            + fsr_right_rear_left
            + fsr_right_rear_right;

        Ok(if left_sum > right_sum {
            Self::Left
        } else {
            Self::Right
        })
    }
}

fn extract_value_from_fsr(fsr: &Value, key: &str) -> anyhow::Result<f64> {
    match fsr.get(key).ok_or_else(|| anyhow!("fsr.get(\"{key}\")"))? {
        Value::Number(value) => Ok(value
            .as_f64()
            .ok_or_else(|| anyhow!("fsr.get(\"{key}\") is not a floating point number"))?),
        _ => bail!("fsr.get(\"{key}\") is not a number"),
    }
}
