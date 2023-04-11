use std::{ops::Div, time::Duration};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::Joints;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct JointsTime {
    joint_times: Joints,
}

impl JointsTime {
    fn abs_time(&self) -> Vec<Vec<f32>> {
        let joint_times = self.joint_times.raw();
        joint_times
            .into_iter()
            .map(|body_part_times| body_part_times.into_iter().map(f32::abs).collect())
            .collect()
    }

    pub fn max(&self) -> Duration {
        let maximum_time: f32 = self
            .abs_time()
            .into_iter()
            .flatten()
            .reduce(|acc, e| f32::max(e, acc))
            .unwrap();
        Duration::from_secs_f32(maximum_time)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct JointsVelocity {
    joint_velocities: Joints,
}

impl Div<JointsVelocity> for Joints {
    type Output = JointsTime;

    fn div(self, rhs: JointsVelocity) -> Self::Output {
        JointsTime {
            joint_times: self / rhs.joint_velocities,
        }
    }
}
