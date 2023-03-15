use splines::{Interpolation, Key, Spline};
use thiserror::Error;

use crate::{Joints, MotionFile};
use std::time::Duration;

pub struct SplineMotionFileInterpolator {
    interpolator: Spline<f32, Joints>,
    current_time: Duration,
    start_time: Duration,
    end_time: Duration,
}

#[derive(Error, Debug)]
pub enum InterpolatorError {
    #[error("accessed interpolator defined for [{min}s, {max}s] at {current}s")]
    OutOfBoundsAccess { min: f32, max: f32, current: f32 },
}

impl From<MotionFile> for SplineMotionFileInterpolator {
    fn from(motion_file: MotionFile) -> Self {
        assert!(!motion_file.frames.is_empty());

        let mut current_time = Duration::ZERO;
        let mut keys = vec![Key::new(
            current_time,
            motion_file.initial_positions,
            Interpolation::Linear,
        )];

        keys.extend(motion_file.frames.into_iter().map(|frame| {
            current_time += frame.duration;
            Key::new(current_time, frame.positions, Interpolation::Linear)
        }));

        Self::new(keys)
    }
}

impl SplineMotionFileInterpolator {
    pub fn new(keys: Vec<Key<Duration, Joints>>) -> Self {
        assert!(keys.len() >= 2, "need at least two keys to interpolate");
        //NOTE: assume keys are sorted
        let last_key_index = keys.len() - 1;

        let start_time = keys[0].t;
        let current_time = start_time;
        let end_time = keys[last_key_index].t;

        let left_helper = Key::new(
            2. * keys[0].t.as_secs_f32() - keys[1].t.as_secs_f32(),
            keys[1].value,
            Interpolation::Linear,
        );
        let right_helper = Key::new(
            2. * keys[last_key_index].t.as_secs_f32() - keys[last_key_index - 1].t.as_secs_f32(),
            keys[last_key_index - 1].value,
            Interpolation::Linear,
        );

        let mut interpolator = Spline::from_iter(
            keys.into_iter()
                .map(|key| Key::new(key.t.as_secs_f32(), key.value, Interpolation::Linear)),
        );
        interpolator.add(left_helper);
        interpolator.add(right_helper);

        Self {
            interpolator,
            current_time,
            start_time,
            end_time,
        }
    }

    pub fn advance_by(&mut self, time_step: Duration) {
        self.current_time += time_step;
    }

    pub fn value(&self) -> Result<Joints, InterpolatorError> {
        if self.current_time <= self.start_time {
            self.interpolator.keys().iter().nth(1).map(|key| key.value)
        } else if self.current_time >= self.end_time {
            self.interpolator
                .keys()
                .iter()
                .rev()
                .nth(1)
                .map(|key| key.value)
        } else {
            self.interpolator.sample(self.current_time.as_secs_f32())
        }
        .ok_or(InterpolatorError::OutOfBoundsAccess {
            min: self.start_time.as_secs_f32(),
            max: self.end_time.as_secs_f32(),
            current: self.current_time.as_secs_f32(),
        })
    }

    pub fn is_finished(&self) -> bool {
        self.current_time >= self.end_time
    }

    pub fn reset(&mut self) {
        self.current_time = self.start_time;
    }
}
