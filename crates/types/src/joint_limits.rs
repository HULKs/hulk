//! Shared serial-joint hardware limits. Controller gains remain node parameters.

use kinematics::joints::Joints;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[serde(deny_unknown_fields)]
pub struct JointLimits {
    /// Inclusive serial joint position bounds in radians.
    pub position: Joints<[f32; 2]>,
    /// Maximum effort from Booster's K1 URDF, in Nm. This is configuration data;
    /// publishing it does not configure firmware or limit the internal PD torque.
    pub maximum_torque: Joints<f32>,
}

impl JointLimits {
    pub fn validate(&self) -> Result<(), String> {
        for (joint, [minimum, maximum]) in self.position.enumerate() {
            if !minimum.is_finite() || !maximum.is_finite() || minimum >= maximum {
                return Err(format!(
                    "invalid joint limits for {joint:?}: [{minimum}, {maximum}]"
                ));
            }
            let torque = self.maximum_torque[joint];
            if !torque.is_finite() || torque <= 0.0 {
                return Err(format!("invalid maximum torque for {joint:?}: {torque}"));
            }
        }
        Ok(())
    }
}
