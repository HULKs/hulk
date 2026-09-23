use crate::joints::Joints;
use std::time::Duration;

pub type JointsVelocity = Joints<f32>;
pub type JointsTime = Joints<Duration>;

impl JointsTime {
    pub fn max(&self) -> Duration {
        self.into_iter()
            .fold(Duration::ZERO, |highest_time, current_time| {
                *current_time.max(&highest_time)
            })
    }
}
