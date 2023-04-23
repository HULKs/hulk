use std::{fs::File, path::Path, time::Duration};

use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::from_reader;

use types::Joints;

use crate::{condition::ConditionEnum};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MotionFile {
    pub initial_positions: Joints<f32>,
    pub frames: Vec<MotionFileFrame>,
}

impl MotionFile {
    pub fn from_path(motion_file_path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(&motion_file_path).wrap_err_with(|| {
            format!("failed to open motion file {:?}", motion_file_path.as_ref())
        })?;
        from_reader(file).wrap_err_with(|| {
            format!(
                "failed to parse motion file {:?}",
                motion_file_path.as_ref()
            )
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MotionFileFrame {
    Joints {
        #[serde(
            serialize_with = "serialize_float_seconds",
            deserialize_with = "deserialize_float_seconds"
        )]
        duration: Duration,
        positions: Joints<f32>,
    },
    Condition(ConditionEnum),
}

fn serialize_float_seconds<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f32(duration.as_secs_f32())
}

fn deserialize_float_seconds<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Duration::from_secs_f32(f32::deserialize(deserializer)?))
}
