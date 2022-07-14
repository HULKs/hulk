use std::{
    ops::{Add, Mul},
    time::Duration,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LinearInterpolator<T> {
    start_value: T,
    end_value: T,
    duration: Duration,
    argument: f32,
}

#[allow(dead_code)]
impl<T> LinearInterpolator<T>
where
    T: Copy + Mul<f32>,
    <T as Mul<f32>>::Output: Add<<T as Mul<f32>>::Output, Output = T>,
{
    pub fn new(start_value: T, end_value: T, duration: Duration) -> Self {
        Self {
            start_value,
            end_value,
            duration,
            argument: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.argument = 0.0;
    }

    pub fn step(&mut self, time_step: Duration) -> T {
        if self.duration.is_zero() {
            // prevent division by zero
            return self.end_value;
        }

        self.argument =
            (self.argument + time_step.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0);

        self.value()
    }

    pub fn value(&self) -> T {
        self.start_value * (1.0 - self.argument) + self.end_value * self.argument
    }

    pub fn is_finished(&self) -> bool {
        self.duration.is_zero() || self.argument >= 1.0
    }

    pub fn remaining_duration(&self) -> Duration {
        self.duration.mul_f32(1.0 - self.argument)
    }

    pub fn passed_duration(&self) -> Duration {
        self.duration.mul_f32(self.argument)
    }
}

#[cfg(test)]
mod test {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn reset_linear_interpolator() {
        let a = LinearInterpolator::new(1.0, 2.0, Duration::from_secs(5));
        let mut b = LinearInterpolator::new(1.0, 2.0, Duration::from_secs(5));
        b.step(Duration::from_secs(1));
        b.reset();
        assert_eq!(a.duration, b.duration);
    }

    #[test]
    fn step_with_identity_function() {
        let mut linear_interpolator = LinearInterpolator::new(0.0, 5.0, Duration::from_secs(5));
        for time_step in 1..6 {
            let value = linear_interpolator.step(Duration::from_secs(1));
            assert_relative_eq!(time_step as f32, value);
        }
    }

    #[test]
    fn finished_with_argument_larger_or_equal_to_one() {
        let mut linear_interpolator = LinearInterpolator::new(0.0, 1.0, Duration::from_secs(1));
        linear_interpolator.step(Duration::from_secs(1));
        assert!(linear_interpolator.is_finished());
        linear_interpolator.reset();
        linear_interpolator.step(Duration::from_millis(1100));
        assert!(linear_interpolator.is_finished());
    }
}
