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

#[cfg(test)]
mod test {

    #[test]
    fn greater_than_with_hysteresis_from_tresholds() {
        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(false, 0.0, 1.0, 2.0),
            false
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(false, 1.0, 1.0, 2.0),
            false
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(false, 1.5, 1.0, 2.0),
            false
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(false, 2.0, 1.0, 2.0),
            false
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(false, 3.0, 1.0, 2.0),
            true
        );

        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(true, 0.0, 1.0, 2.0),
            false
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(true, 1.0, 1.0, 2.0),
            true
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(true, 1.5, 1.0, 2.0),
            true
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(true, 2.0, 1.0, 2.0),
            true
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_tresholds(true, 3.0, 1.0, 2.0),
            true
        );
    }

    #[test]
    fn less_than_with_hysteresis_from_thresholds() {
        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(false, 0.0, 1.0, 2.0),
            true
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(false, 1.0, 1.0, 2.0),
            false
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(false, 1.5, 1.0, 2.0),
            false
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(false, 2.0, 1.0, 2.0),
            false
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(false, 3.0, 1.0, 2.0),
            false
        );

        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(true, 0.0, 1.0, 2.0),
            true
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(true, 1.0, 1.0, 2.0),
            true
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(true, 1.5, 1.0, 2.0),
            true
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(true, 2.0, 1.0, 2.0),
            true
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_thresholds(true, 3.0, 1.0, 2.0),
            false
        );
    }

    #[test]
    fn greater_than_with_hysteresis_from_delta() {
        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(false, 0.0, 1.0, 0.5),
            false
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(false, 1.0, 1.0, 0.5),
            false
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(false, 1.5, 1.0, 0.5),
            false
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(false, 2.0, 1.0, 0.5),
            true
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(false, 2.5, 1.0, 0.5),
            true
        );

        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(true, 0.0, 1.0, 0.5),
            false
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(true, 1.0, 1.0, 0.5),
            true
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(true, 1.5, 1.0, 0.5),
            true
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(true, 2.0, 1.0, 0.5),
            true
        );
        assert_eq!(
            super::greater_than_with_hysteresis_from_delta(true, 2.5, 1.0, 0.5),
            true
        );
    }

    #[test]
    fn less_than_with_hysteresis_from_delta() {
        assert_eq!(
            super::less_than_with_hysteresis_from_delta(false, 0.0, 1.0, 0.5),
            true
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_delta(false, 1.0, 1.0, 0.5),
            false
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_delta(false, 1.5, 1.0, 0.5),
            false
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_delta(false, 2.0, 1.0, 0.5),
            false
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_delta(false, 2.5, 1.0, 0.5),
            false
        );

        assert_eq!(
            super::less_than_with_hysteresis_from_delta(true, 0.0, 1.0, 0.5),
            true
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_delta(true, 1.0, 1.0, 0.5),
            true
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_delta(true, 1.5, 1.0, 0.5),
            true
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_delta(true, 2.0, 1.0, 0.5),
            false
        );
        assert_eq!(
            super::less_than_with_hysteresis_from_delta(true, 2.5, 1.0, 0.5),
            false
        );
    }
}
