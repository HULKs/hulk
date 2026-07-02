use factrs::linalg::Numeric;
use nalgebra::SVector;

#[derive(Debug)]
pub struct CubicHermiteSpline<T: Numeric, const D: usize> {
    start: SVector<T, D>,
    end: SVector<T, D>,
    start_derivative: SVector<T, D>,
    end_derivative: SVector<T, D>,
}

impl<T: Numeric, const D: usize> CubicHermiteSpline<T, D> {
    pub fn new(
        start: SVector<T, D>,
        end: SVector<T, D>,
        start_derivative: SVector<T, D>,
        end_derivative: SVector<T, D>,
    ) -> Self {
        Self {
            start,
            end,
            start_derivative,
            end_derivative,
        }
    }

    /// Evaluate the cubic Hermite spline at the given tau value.
    /// tau should be in the range [0, 1].
    pub fn evaluate(&self, tau: T) -> SVector<T, D> {
        // https://en.wikipedia.org/wiki/Cubic_Hermite_spline#Unit_interval_[0,_1]
        let tau_squared = tau * tau;
        let tau_cubed = tau_squared * tau;
        let one = T::one();
        let two = T::from(2.0);
        let three = T::from(3.0);

        let c1 = two * tau_cubed - three * tau_squared + one;
        let c2 = tau_cubed - two * tau_squared + tau;
        let c3 = -two * tau_cubed + three * tau_squared;
        let c4 = tau_cubed - tau_squared;

        self.start * c1 + self.start_derivative * c2 + self.end * c3 + self.end_derivative * c4
    }

    pub fn evaluate_derivative(&self, tau: T) -> SVector<T, D> {
        let tau_squared = tau * tau;
        let one = T::one();
        let two = T::from(2.0);
        let three = T::from(3.0);
        let four = T::from(4.0);
        let six = T::from(6.0);

        let c1 = six * tau_squared - six * tau;
        let c2 = three * tau_squared - four * tau + one;
        let c3 = -six * tau_squared + six * tau;
        let c4 = three * tau_squared - two * tau;

        self.start * c1 + self.start_derivative * c2 + self.end * c3 + self.end_derivative * c4
    }

    pub fn evaluate_second_derivative(&self, tau: T) -> SVector<T, D> {
        let two = T::from(2.0);
        let four = T::from(4.0);
        let six = T::from(6.0);
        let twelve = T::from(12.0);

        let c1 = twelve * tau - six;
        let c2 = six * tau - four;
        let c3 = -twelve * tau + six;
        let c4 = six * tau - two;

        self.start * c1 + self.start_derivative * c2 + self.end * c3 + self.end_derivative * c4
    }
}
