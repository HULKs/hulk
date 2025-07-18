use std::ops::Neg;

/// The logistic function σ(x) = 1 / (1 + e^(-x))
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + x.neg().exp())
}
