use std::{ops::{Deref, DerefMut}, time::Duration};
use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use splines::{Interpolation, Key, Interpolate};
use types::{Joints, JointsVelocity};

use crate::spline_interpolator::SplineInterpolator;

use color_eyre::{eyre::Context, Result};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct TransitionInterpolator<T: Debug + Interpolate<f32>> {
    #[serde(skip)]
    interpolator: SplineInterpolator<T>,
}

impl TransitionInterpolator<Joints> {
    pub fn try_new_with_maximum_velocity(
        current_position: Joints,
        target_position: Joints,
        maximum_velocity: JointsVelocity,
    ) -> Result<TransitionInterpolator<Joints>> {
        let time_to_completion = (target_position - current_position) / maximum_velocity;
        let maximum_time_to_completion = time_to_completion.max();

        Self::try_new_timed(current_position, target_position, maximum_time_to_completion)
    }
}

impl<T: Debug + Interpolate<f32>> TransitionInterpolator<T> {
    pub fn try_new_timed(current_position: T, target_position: T, duration: Duration) -> Result<TransitionInterpolator<T>> {
        let keys = vec![
            Key::new(Duration::ZERO, current_position, Interpolation::Linear),
            Key::new(
                duration,
                target_position,
                Interpolation::Linear,
            ),
        ];

        Ok(Self {
            interpolator: SplineInterpolator::try_new(keys)
                .wrap_err("failed to create TransitionInterpolator")?,
        })
    }
}

impl<T: Debug + Interpolate<f32>> Deref for TransitionInterpolator<T> {
    type Target = SplineInterpolator<T>;

    fn deref(&self) -> &Self::Target {
        &self.interpolator
    }
}
impl<T: Debug + Interpolate<f32>> DerefMut for TransitionInterpolator<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.interpolator
    }
}
