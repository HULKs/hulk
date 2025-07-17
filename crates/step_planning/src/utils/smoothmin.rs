pub fn smoothmin(x: f32, max: f32, smoothness: f32) -> f32 {
    match x {
        x if x < max - smoothness => x + smoothness * 0.5,
        x if x < max => max - (max - x).powi(2) / (smoothness * 2.0),
        _x => max,
    }
}

pub fn smoothmin_derivative(x: f32, max: f32, smoothness: f32) -> f32 {
    match x {
        x if x < max - smoothness => 1.0,
        x if x < max => 2.0 * (max - x) / (smoothness * 2.0),
        _x => 0.0,
    }
}
