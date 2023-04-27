use crate::KeyFrame;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use splines::{Interpolate, Interpolation, Key, Spline};
use thiserror::Error;
use types::{Joints, JointsVelocity};

use std::{fmt::Debug, time::Duration};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimedSpline<T> {
    spline: Spline<f32, T>,
    total_duration: Duration,
}

impl<T> Default for TimedSpline<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            spline: Spline::from_vec(vec![]),
            total_duration: Duration::ZERO,
        }
    }
}

pub trait MapArgumentExt<FromArgument, ToArgument, Value> {
    fn map_argument(self) -> Result<Interpolation<ToArgument, Value>, InterpolatorError>;
}

impl<FromArgument: Debug, ToArgument, Joints: Debug>
    MapArgumentExt<FromArgument, ToArgument, Joints> for Interpolation<FromArgument, Joints>
{
    fn map_argument(self) -> Result<Interpolation<ToArgument, Joints>, InterpolatorError> {
        match self {
            Interpolation::Linear => Ok(Interpolation::Linear),
            Interpolation::Cosine => Ok(Interpolation::Cosine),
            Interpolation::CatmullRom => Ok(Interpolation::CatmullRom),
            unimplemented_mode => Err(InterpolatorError::UnsupportedInterpolationMode {
                interpolation_mode: format!("{unimplemented_mode:?}"),
            }),
        }
    }
}

#[derive(Error, Debug)]
pub enum InterpolatorError {
    #[error("cannot perform {interpolation_mode} with {keys_before} keys before and {keys_after} keys after")]
    InterpolationControlKey {
        interpolation_mode: String,
        keys_before: usize,
        keys_after: usize,
    },
    #[error("need at least two keys to create an interpolator")]
    NotEnoughKeys,
    #[error("uses unsupported interpolation mode {interpolation_mode}")]
    UnsupportedInterpolationMode { interpolation_mode: String },
    #[error("the time value is not monotonically increasing")]
    KeysTimeIncorrect,
}

impl InterpolatorError {
    fn create_control_key_error<T: Debug>(
        keys: &[Key<f32, T>],
        current_time: Duration,
    ) -> InterpolatorError {
        let current_control_key = keys
            .iter()
            .filter(|key| key.t <= current_time.as_secs_f32())
            .last()
            .unwrap();

        let prior_control_points = keys
            .iter()
            .take_while(|key| key.t != current_control_key.t)
            .count();
        let following_control_points = keys.len() - 1 - prior_control_points;

        InterpolatorError::InterpolationControlKey {
            interpolation_mode: format!("{:?}", current_control_key.interpolation),
            keys_before: prior_control_points,
            keys_after: following_control_points,
        }
    }
}

impl TimedSpline<Joints<f32>> {
    pub fn try_new_transition_with_velocity(
        current_position: Joints<f32>,
        target_position: Joints<f32>,
        maximum_velocity: JointsVelocity,
    ) -> Result<TimedSpline<Joints<f32>>, InterpolatorError> {
        let time_to_completion = (target_position - current_position) / maximum_velocity;
        let maximum_time_to_completion = time_to_completion.max();

        Self::try_new_transition_timed(
            current_position,
            target_position,
            maximum_time_to_completion,
        )
    }
}

impl<T> TimedSpline<T>
where
    T: Debug + Interpolate<f32>,
{
    pub fn try_new_with_start(
        initial_position: T,
        keys: Vec<KeyFrame<T>>,
    ) -> Result<Self, InterpolatorError> {
        let mut time_since_start = Duration::ZERO;

        let mut spline_keys = vec![Key::new(
            time_since_start,
            initial_position,
            Interpolation::Linear,
        )];
        spline_keys.extend(
            keys.into_iter()
                .map(|frame| {
                    time_since_start += frame.duration;
                    Ok(Key::new(
                        time_since_start,
                        frame.positions,
                        Interpolation::Linear,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );

        Self::try_new(spline_keys)
    }

    pub fn try_new(keys: Vec<Key<Duration, T>>) -> Result<Self, InterpolatorError> {
        if keys.len() < 2 {
            return Err(InterpolatorError::NotEnoughKeys);
        }

        if keys
            .iter()
            .tuple_windows()
            .any(|(first_frame, second_frame)| first_frame.t >= second_frame.t)
        {
            return Err(InterpolatorError::KeysTimeIncorrect);
        }

        let start_time = keys.first().unwrap().t;
        let end_time = keys.last().unwrap().t - start_time;
        let last_key_index = keys.len() - 1;

        let mut spline = Spline::from_vec(
            keys.into_iter()
                .map(|key| {
                    Ok(Key::new(
                        key.t.as_secs_f32() - start_time.as_secs_f32(),
                        key.value,
                        key.interpolation.map_argument()?,
                    ))
                })
                .collect::<Result<_, _>>()?,
        );

        spline.add(Self::create_zero_gradient(
            &spline.keys()[last_key_index],
            &spline.keys()[last_key_index - 1],
        ));
        spline.add(Self::create_zero_gradient(
            &spline.keys()[0],
            &spline.keys()[1],
        ));

        Ok(Self {
            spline,
            total_duration: end_time,
        })
    }

    pub fn try_new_transition_timed(
        current_position: T,
        target_position: T,
        duration: Duration,
    ) -> Result<TimedSpline<T>, InterpolatorError> {
        let keys = vec![
            Key::new(Duration::ZERO, current_position, Interpolation::CatmullRom),
            Key::new(duration, target_position, Interpolation::CatmullRom),
        ];

        Self::try_new(keys)
    }

    fn create_zero_gradient(key_center: &Key<f32, T>, key_other: &Key<f32, T>) -> Key<f32, T> {
        Key::new(
            2. * key_center.t - key_other.t,
            key_other.value,
            key_center.interpolation,
        )
    }

    pub fn value_at(&self, time_point: Duration) -> T {
        if time_point >= self.total_duration {
            return self.end_position();
        }
        // Duration and f32 have different precisions, we have to ensure that if self.current_duration < self.total_duration, that
        // self.current_duration.as_secs_f32() != self.total_duration.as_secs_f32(), since otherwise we are unable to sample the spline.
        let clamped_time_point = time_point
            .as_secs_f32()
            .clamp(0., self.total_duration.as_secs_f32() - f32::EPSILON);
        self.spline
            .sample(clamped_time_point)
            .ok_or_else(|| {
                InterpolatorError::create_control_key_error(self.spline.keys(), time_point)
            })
            .expect("could not sample spline")
    }

    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }

    // TODO: uses weird indexing due to the artificial keys added in the try_new function
    // if possible in the future, use spline boundary conditions instead.
    pub fn start_position(&self) -> T {
        self.spline.keys()[1].value
    }

    pub fn end_position(&self) -> T {
        self.spline.keys()[self.spline.keys().len() - 2].value
    }

    pub fn set_initial_positions(&mut self, position: T) {
        if let Some(key) = self.spline.get_mut(1) {
            *key.value = position;
        }
    }
}
