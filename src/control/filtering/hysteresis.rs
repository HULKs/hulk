pub fn greater_than_with_hysteresis(
    last_evaluation: bool,
    value: f32,
    threshold: f32,
    hysteresis: f32,
) -> bool {
    value
        > threshold
            + if last_evaluation {
                -hysteresis
            } else {
                hysteresis
            }
}
