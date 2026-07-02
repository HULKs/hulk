use factrs::{linalg::VectorX, traits::Variable};

pub struct GeodesicSpline<V: Variable> {
    start: V,
    phi: VectorX<V::T>,
}

impl<V: Variable> GeodesicSpline<V> {
    pub fn new(start: V, end: &V) -> Self {
        let phi = start.inverse().compose(end).log();

        Self { start, phi }
    }

    pub fn evaluate(&self, tau: V::T) -> V {
        self.start.oplus_right((&self.phi * tau).as_view())
    }

    pub fn evaluate_time_derivative(&self, dt: V::T) -> VectorX<V::T> {
        &self.phi / dt
    }
}
