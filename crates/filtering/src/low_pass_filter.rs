use std::{
    f32::consts::PI,
    ops::{Add, Mul, Sub},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct LowPassFilter<State> {
    alpha: f32,
    state: State,
}

impl<State> LowPassFilter<State>
where
    State: Copy + Add<Output = State> + Sub<Output = State> + Mul<f32, Output = State>,
{
    pub fn with_alpha(initial_state: State, alpha: f32) -> Self {
        Self {
            alpha,
            state: initial_state,
        }
    }

    #[allow(dead_code)]
    pub fn with_cutoff(initial_state: State, cutoff_frequency: f32, sampling_rate: f32) -> Self {
        let rc = 1.0 / (cutoff_frequency * 2.0 * PI);
        let dt = 1.0 / sampling_rate;
        let alpha = dt / (rc + dt);
        Self {
            alpha,
            state: initial_state,
        }
    }

    pub fn update(&mut self, value: State) {
        self.state = self.state + (value - self.state) * self.alpha;
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn reset(&mut self, state: State) {
        self.state = state;
    }

    pub fn alpha(&self) -> f32 {
        self.alpha
    }
}
