//! Typed motion configuration edited in the UI and applied as a session-only ROS-Z layer.
use std::{fs, sync::Arc};

use color_eyre::{
    Result,
    eyre::{WrapErr, eyre},
};
use ros_z::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tempfile::TempDir;
use types::joint_limits::JointLimits;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionParameters {
    pub head_motion: head_motion::parameters::Parameters,
    pub motion_inference: motion_inference::config::Parameters,
    pub hardware_interface: hardware_interface::Parameters,
    pub joint_limits: JointLimits,
}

impl MotionParameters {
    pub async fn load(context: &Context) -> Result<Self> {
        Ok(Self {
            head_motion: load(context, "head_motion").await?,
            motion_inference: load(context, "motion_inference").await?,
            hardware_interface: load(context, "hardware_interface").await?,
            joint_limits: load::<global_parameter_provider::Parameters>(context, "global")
                .await?
                .joint_limits,
        })
    }

    pub fn validate(&self) -> Result<()> {
        self.head_motion
            .validate()
            .map_err(|e| eyre!("head_motion: {e}"))?;
        self.motion_inference
            .validate()
            .map_err(|e| eyre!("motion_inference: {e}"))?;
        self.joint_limits
            .validate()
            .map_err(|e| eyre!("joint_limits: {e}"))?;
        for (name, duration) in [
            (
                "joint_control_message_interval",
                self.hardware_interface.joint_control_message_interval,
            ),
            (
                "rotate_head_message_interval",
                self.hardware_interface.rotate_head_message_interval,
            ),
            (
                "sdk_request_timeout",
                self.hardware_interface.sdk_request_timeout,
            ),
        ] {
            if duration.is_zero() {
                return Err(eyre!("hardware_interface.{name} must be positive"));
            }
        }
        Ok(())
    }

    /// Prepare the entire layer before stopping any node. A failed edit cannot partially
    /// update the running stack, and each node reads the same layer after restarting.
    pub fn prepare(value: Value) -> Result<Arc<TempDir>> {
        let parameters: Self =
            serde_json::from_value(value).wrap_err("Invalid motion parameters")?;
        parameters.validate()?;
        for policy in motion_inference::config::Policy::ALL {
            let path = parameters
                .motion_inference
                .neural_networks_folder
                .join(policy.file(&parameters.motion_inference));
            fs::File::open(&path)
                .wrap_err_with(|| format!("Cannot read model {}", path.display()))?;
        }
        let layer = Arc::new(tempfile::tempdir()?);
        for (key, value) in [
            ("head_motion", serde_json::to_value(parameters.head_motion)?),
            (
                "motion_inference",
                serde_json::to_value(parameters.motion_inference)?,
            ),
            (
                "hardware_interface",
                serde_json::to_value(parameters.hardware_interface)?,
            ),
            ("global", json!({"joint_limits": parameters.joint_limits})),
        ] {
            fs::write(
                layer.path().join(format!("{key}.json5")),
                serde_json::to_vec_pretty(&value)?,
            )?;
        }
        Ok(layer)
    }
}

async fn load<T>(context: &Context, key: &str) -> Result<T>
where
    T: Clone + Serialize + DeserializeOwned + ros_z::Message + Send + Sync + 'static,
{
    let node = context
        .create_node(format!("simulator_edit_{key}"))
        .build()
        .await?;
    let binding = node.bind_parameter_as::<T>(key)?;
    Ok(binding.snapshot().typed().clone())
}
