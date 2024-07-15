use std::ops::RangeInclusive;

pub fn greater_than_with_absolute_hysteresis(
    last_evaluation: bool,
    value: f32,
    hysteresis: RangeInclusive<f32>,
) -> bool {
    if hysteresis.contains(&value) {
        last_evaluation
    } else {
        value > *hysteresis.end()
    }
}

#[allow(clippy::if_same_then_else)]
pub fn less_than_with_absolute_hysteresis(
    last_evaluation: bool,
    value: f32,
    hysteresis: RangeInclusive<f32>,
) -> bool {
    if hysteresis.contains(&value) {
        last_evaluation
    } else {
        value < *hysteresis.start()
    }
}

pub fn greater_than_with_relative_hysteresis(
    last_evaluation: bool,
    value: f32,
    threshold: f32,
    hysteresis: RangeInclusive<f32>,
) -> bool {
    assert!(*hysteresis.start() <= 0.0);
    assert!(*hysteresis.end() >= 0.0);

    greater_than_with_absolute_hysteresis(
        last_evaluation,
        value,
        threshold + *hysteresis.start()..=threshold + *hysteresis.end(),
    )
}

pub fn less_than_with_relative_hysteresis(
    last_evaluation: bool,
    value: f32,
    threshold: f32,
    hysteresis: RangeInclusive<f32>,
) -> bool {
    assert!(*hysteresis.start() <= 0.0);
    assert!(*hysteresis.end() >= 0.0);

    less_than_with_absolute_hysteresis(
        last_evaluation,
        value,
        threshold + *hysteresis.start()..=threshold + *hysteresis.end(),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn greater_than_with_hysteresis_from_tresholds() {
        assert!(!greater_than_with_absolute_hysteresis(
            false,
            0.0,
            1.0..=2.0
        ));
        assert!(!greater_than_with_absolute_hysteresis(
            false,
            1.0,
            1.0..=2.0
        ));
        assert!(!greater_than_with_absolute_hysteresis(
            false,
            1.5,
            1.0..=2.0
        ));
        assert!(!greater_than_with_absolute_hysteresis(
            false,
            2.0,
            1.0..=2.0
        ));
        assert!(greater_than_with_absolute_hysteresis(false, 3.0, 1.0..=2.0));

        assert!(!greater_than_with_absolute_hysteresis(true, 0.0, 1.0..=2.0));
        assert!(greater_than_with_absolute_hysteresis(true, 1.0, 1.0..=2.0));
        assert!(greater_than_with_absolute_hysteresis(true, 1.5, 1.0..=2.0));
        assert!(greater_than_with_absolute_hysteresis(true, 2.0, 1.0..=2.0));
        assert!(greater_than_with_absolute_hysteresis(true, 3.0, 1.0..=2.0));
    }

    #[test]
    fn less_than_with_hysteresis_from_thresholds() {
        assert!(less_than_with_absolute_hysteresis(false, 0.0, 1.0..=2.0));
        assert!(!less_than_with_absolute_hysteresis(false, 1.0, 1.0..=2.0));
        assert!(!less_than_with_absolute_hysteresis(false, 1.5, 1.0..=2.0));
        assert!(!less_than_with_absolute_hysteresis(false, 2.0, 1.0..=2.0));
        assert!(!less_than_with_absolute_hysteresis(false, 3.0, 1.0..=2.0));

        assert!(less_than_with_absolute_hysteresis(true, 0.0, 1.0..=2.0));
        assert!(less_than_with_absolute_hysteresis(true, 1.0, 1.0..=2.0));
        assert!(less_than_with_absolute_hysteresis(true, 1.5, 1.0..=2.0));
        assert!(less_than_with_absolute_hysteresis(true, 2.0, 1.0..=2.0));
        assert!(!less_than_with_absolute_hysteresis(true, 3.0, 1.0..=2.0));
    }

    #[test]
    fn greater_than_with_hysteresis_from_delta() {
        assert!(!greater_than_with_relative_hysteresis(
            false,
            0.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(!greater_than_with_relative_hysteresis(
            false,
            1.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(greater_than_with_relative_hysteresis(
            false,
            1.5,
            1.0,
            -0.25..=0.25
        ));
        assert!(greater_than_with_relative_hysteresis(
            false,
            2.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(greater_than_with_relative_hysteresis(
            false,
            2.5,
            1.0,
            -0.25..=0.25
        ));

        assert!(!greater_than_with_relative_hysteresis(
            true,
            0.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(greater_than_with_relative_hysteresis(
            true,
            1.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(greater_than_with_relative_hysteresis(
            true,
            1.5,
            1.0,
            -0.25..=0.25
        ));
        assert!(greater_than_with_relative_hysteresis(
            true,
            2.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(greater_than_with_relative_hysteresis(
            true,
            2.5,
            1.0,
            -0.25..=0.25
        ));
    }

    #[test]
    fn less_than_with_hysteresis_from_delta() {
        assert!(less_than_with_relative_hysteresis(
            false,
            0.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(!less_than_with_relative_hysteresis(
            false,
            1.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(!less_than_with_relative_hysteresis(
            false,
            1.5,
            1.0,
            -0.25..=0.25
        ));
        assert!(!less_than_with_relative_hysteresis(
            false,
            2.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(!less_than_with_relative_hysteresis(
            false,
            2.5,
            1.0,
            -0.25..=0.25
        ));

        assert!(less_than_with_relative_hysteresis(
            true,
            0.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(less_than_with_relative_hysteresis(
            true,
            1.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(!less_than_with_relative_hysteresis(
            true,
            1.5,
            1.0,
            -0.25..=0.25
        ));
        assert!(!less_than_with_relative_hysteresis(
            true,
            2.0,
            1.0,
            -0.25..=0.25
        ));
        assert!(!less_than_with_relative_hysteresis(
            true,
            2.5,
            1.0,
            -0.25..=0.25
        ));
    }
}
