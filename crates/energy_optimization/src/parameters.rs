use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
pub struct CurrentMinimizerParameters {
    pub allowed_current: f32,
    pub allowed_current_upper_threshold: f32,
    pub optimization_speed: f32,
}
