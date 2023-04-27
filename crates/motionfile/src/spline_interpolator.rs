use std::fmt::Debug;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use splines::Interpolate;

use crate::TimedSpline;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SplineInterpolator<T> {
    spline: TimedSpline<T>,
    current_duration: Duration,
}

impl<T: Debug + Interpolate<f32>> SplineInterpolator<T> {
    pub fn advance_by(&mut self, time_step: Duration) {
        self.current_duration += time_step
    }

    pub fn value(&self) -> T {
        self.spline.value_at(self.current_duration)
    }

    pub fn current_duration(&self) -> Duration {
        self.current_duration
    }

    pub fn set_initial_positions(&mut self, position: T) {
        self.spline.set_initial_positions(position);
    }

    pub fn is_finished(&self) -> bool {
        self.current_duration >= self.spline.total_duration()
    }

    pub fn total_duration(&self) -> Duration {
        self.spline.total_duration()
    }
}

impl<T> From<TimedSpline<T>> for SplineInterpolator<T> {
    fn from(spline: TimedSpline<T>) -> Self {
        Self {
            spline,
            current_duration: Duration::ZERO,
        }
    }
}
