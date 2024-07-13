pub fn greater_than_with_hysteresis_from_delta(
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

pub fn less_than_with_hysteresis_from_delta(
    last_evaluation: bool,
    value: f32,
    threshold: f32,
    hysteresis: f32,
) -> bool {
    value
        < threshold
            + if last_evaluation {
                hysteresis
            } else {
                -hysteresis
            }
}

pub fn greater_than_with_hysteresis_from_tresholds(
    last_evaluation: bool,
    value: f32,
    lower_threshold: f32,
    upper_threshold: f32,
) -> bool {
    if last_evaluation {
        value > lower_threshold
    } else {
        value > upper_threshold
    }
}

pub fn less_than_with_hysteresis_from_thresholds(
    last_evaluation: bool,
    value: f32,
    lower_threshold: f32,
    upper_threshold: f32,
) -> bool {
    if last_evaluation {
        value < upper_threshold
    } else {
        value < lower_threshold
    }
}
