use crate::Joints;
use std::time::Duration;

pub type JointsVelocity = Joints<f32>;
pub type JointsTime = Joints<Duration>;

impl JointsTime {
    pub fn max(&self) -> Duration {
        self.as_vec()
            .into_iter()
            .flatten()
            .reduce(|acc, e| Duration::max(e, acc))
            .unwrap()
    }
}
