use serde::{Deserialize, Serialize};
use splines::{Interpolate, Interpolation, Key, Spline};
use thiserror::Error;
use types::{Joints, JointsVelocity, MotionFile};

use std::{fmt::Debug, time::Duration};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SplineInterpolator<T> {
    spline: Spline<f32, T>,
    current_duration: Duration,
    total_duration: Duration,
}

impl<T> Default for SplineInterpolator<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            spline: Spline::from_vec(vec![]),
            current_duration: Duration::ZERO,
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

impl TryFrom<MotionFile> for SplineInterpolator<Joints<f32>> {
    type Error = InterpolatorError;

    fn try_from(motion_file: MotionFile) -> Result<Self, InterpolatorError> {
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

        SplineInterpolator::try_new(keys)
    }
}

impl SplineInterpolator<Joints<f32>> {
    pub fn try_new_transition_with_velocity(
        current_position: Joints<f32>,
        target_position: Joints<f32>,
        maximum_velocity: JointsVelocity,
    ) -> Result<SplineInterpolator<Joints<f32>>, InterpolatorError> {
        let time_to_completion = (target_position - current_position) / maximum_velocity;
        let maximum_time_to_completion = time_to_completion.max();

        Self::try_new_transition_timed(
            current_position,
            target_position,
            maximum_time_to_completion,
        )
    }
}

impl<T> SplineInterpolator<T>
where
    T: Debug + Interpolate<f32>,
{
    pub fn try_new(mut keys: Vec<Key<Duration, T>>) -> Result<Self, InterpolatorError> {
        if keys.len() < 2 {
            return Err(InterpolatorError::NotEnoughKeys);
        }

        keys.sort_unstable_by_key(|key| key.t);

        let current_time = Duration::ZERO;
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
            current_duration: current_time,
            total_duration: end_time,
        })
    }

    pub fn try_new_transition_timed(
        current_position: T,
        target_position: T,
        duration: Duration,
    ) -> Result<SplineInterpolator<T>, InterpolatorError> {
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

    pub fn advance_by(&mut self, time_step: Duration) {
        self.current_duration += time_step;
    }

    pub fn value(&self) -> Result<T, InterpolatorError> {
        if self.current_duration >= self.total_duration {
            self.spline.keys().iter().rev().nth(1).map(|key| key.value)
        } else {
            // Duration and f32 have different precisions, we have to ensure that if self.current_duration < self.total_duration, that
            // self.current_duration.as_secs_f32() != self.total_duration.as_secs_f32(), since otherwise we are unable to sample the spline.
            let clamped_duration = self
                .current_duration
                .as_secs_f32()
                .clamp(0., self.total_duration.as_secs_f32() - f32::EPSILON);
            self.spline.sample(clamped_duration)
        }
        .ok_or_else(|| {
            InterpolatorError::create_control_key_error(self.spline.keys(), self.current_duration)
        })
    }

    pub fn is_finished(&self) -> bool {
        self.current_duration >= self.total_duration
    }

    pub fn reset(&mut self) {
        self.current_duration = Duration::ZERO;
    }

    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }

    pub fn current_duration(&self) -> Duration {
        self.current_duration
    }

    pub fn set_initial_positions(&mut self, current_position: Joints<f32>) {
        if let Some(initial_key) = self.spline.get_mut(1) {
            *initial_key.value = current_position;
        }
    }
}
