pub fn greater_than_with_hysteresis_from_tresholds(
    last_evaluation: bool,
    value: f32,
    lower_threshold: f32,
    upper_threshold: f32,
) -> bool {
    if value > upper_threshold {
        true
    } else if value < lower_threshold {
        false
    } else {
        last_evaluation
    }
}

#[allow(clippy::if_same_then_else)]
pub fn less_than_with_hysteresis_from_thresholds(
    last_evaluation: bool,
    value: f32,
    lower_threshold: f32,
    upper_threshold: f32,
) -> bool {
    if value < lower_threshold {
        true
    } else if value > upper_threshold {
        false
    } else {
        last_evaluation
    }
}

pub fn greater_than_with_hysteresis_from_delta(
    last_evaluation: bool,
    value: f32,
    threshold: f32,
    hysteresis: f32,
) -> bool {
    greater_than_with_hysteresis_from_tresholds(
        last_evaluation,
        value,
        threshold - hysteresis,
        threshold + hysteresis,
    )
}

pub fn less_than_with_hysteresis_from_delta(
    last_evaluation: bool,
    value: f32,
    threshold: f32,
    hysteresis: f32,
) -> bool {
    less_than_with_hysteresis_from_thresholds(
        last_evaluation,
        value,
        threshold - hysteresis,
        threshold + hysteresis,
    )
}
