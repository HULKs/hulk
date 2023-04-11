use std::{ops::{Deref, DerefMut}, time::Duration};

use splines::{Interpolation, Key};
use types::{Joints, JointsVelocity};

use crate::spline_interpolator::SplineInterpolator;

use color_eyre::{eyre::Context, Result};

#[derive(Default)]
pub struct TransitionInterpolator {
    interpolator: SplineInterpolator,
}

impl TransitionInterpolator {
    pub fn try_new(
        current_position: Joints,
        target_position: Joints,
        maximum_velocity: JointsVelocity,
    ) -> Result<TransitionInterpolator> {
        let time_to_completion = (target_position - current_position) / maximum_velocity;
        let maximum_time_to_completion = time_to_completion.max();

        let keys = vec![
            Key::new(Duration::ZERO, current_position, Interpolation::Linear),
            Key::new(
                maximum_time_to_completion,
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

impl Deref for TransitionInterpolator {
    type Target = SplineInterpolator;

    fn deref(&self) -> &Self::Target {
        &self.interpolator
    }
}
impl DerefMut for TransitionInterpolator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.interpolator
    }
}
