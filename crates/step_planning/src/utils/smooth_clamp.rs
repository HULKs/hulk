/// Smooth (C¹) clamping function.
/// Returns the value and derivative at x.
/// The corners of the `clamp` function at min and max are rounded off
/// quadratically, with the bent part limited to {min,max} ± smoothness.
/// [Desmos link](https://www.desmos.com/calculator/xa1jklgrcg)
pub fn smooth_clamp(x: f64, min: f64, max: f64, smoothness: f64) -> (f64, f64) {
    // "cutoffs", where the piecewise parts are joined
    let c1 = min - smoothness;
    let c2 = min + smoothness;
    let c3 = max - smoothness;
    let c4 = max + smoothness;

    debug_assert!(min <= max);
    debug_assert!(c2 <= c3);

    match x {
        x if x < c1 => (min, 0.0),
        x if x < c2 => (
            min + (x - c1).powi(2) / (4.0 * smoothness),
            (x - c1) / (2.0 * smoothness),
        ),
        x if x < c3 => (x, 1.0),
        x if x < c4 => (
            max - (x - c4).powi(2) / (4.0 * smoothness),
            -(x - c4) / (2.0 * smoothness),
        ),
        _x => (max, 0.0),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use itertools::Itertools;

    use super::*;

    fn f(x: f64) -> (f64, f64) {
        smooth_clamp(x, 2.0, 5.0, 1.0)
    }

    #[test]
    fn test_smooth_clamp() {
        assert_eq!((2.0, 0.0), f(0.0));
        assert_eq!((2.0, 0.0), f(1.0));

        assert_eq!((3.0, 1.0), f(3.0));
        assert_eq!((4.0, 1.0), f(4.0));

        assert_eq!((5.0, 0.0), f(6.0));
        assert_eq!((5.0, 0.0), f(7.0));
    }

    fn sample_interval(min: f64, max: f64, n: usize) -> impl ExactSizeIterator<Item = f64> {
        #[expect(clippy::cast_precision_loss)]
        (0..n).map(move |i| min + i as f64 * (max - min) / n as f64)
    }

    #[test]
    fn test_smooth_clamp_smoothness() {
        sample_interval(0.0, 7.0, 1000)
            .tuple_windows()
            .for_each(|(x1, x2)| {
                let (y1, dy1) = f(x1);
                let (y2, dy2) = f(x2);

                let dy = y2 - y1;
                let dx = x2 - x1;

                let slope = dy / dx;

                assert!((0.0..=1.0).contains(&slope));

                let ddy = dy2 - dy1;
                let approximate_second_derivative = ddy / dx;

                // as defined in `f`
                let smoothness = 1.0;
                let maximum_second_derivative = 1.0 / (2.0 * smoothness);

                assert!((-maximum_second_derivative..=maximum_second_derivative)
                    .contains(&approximate_second_derivative));
            });
    }
}
