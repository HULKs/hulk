fn normalized_smoothstep(x: f64) -> (f64, f64) {
    if x < 0.0 {
        (0.0, 0.0)
    } else if x > 1.0 {
        (1.0, 0.0)
    } else {
        (x * x * (3.0 - 2.0 * x), x * (6.0 - 6.0 * x))
    }
}

pub fn smoothstep(x: f64, min: f64, max: f64) -> (f64, f64) {
    normalized_smoothstep((x - min) / (max - min))
}
