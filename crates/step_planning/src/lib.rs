pub mod cost_fields;
pub mod geometry;
pub mod step_plan;
pub mod traits;
pub mod utils;

#[cfg(test)]
pub mod test_utils {
    use std::f32::consts::FRAC_PI_2;

    use geometry::{arc::Arc, circle::Circle, direction::Direction, line_segment::LineSegment};
    use linear_algebra::{point, Orientation2};
    use types::planned_path::{Path, PathSegment};

    pub fn test_path() -> Path {
        Path {
            segments: vec![
                PathSegment::LineSegment(LineSegment(point![0.0, 0.0], point![3.0, 0.0])),
                PathSegment::Arc(Arc {
                    circle: Circle {
                        center: point![3.0, 1.0],
                        radius: 1.0,
                    },
                    start: Orientation2::new(3.0 * FRAC_PI_2),
                    end: Orientation2::new(0.0),
                    direction: Direction::Counterclockwise,
                }),
                PathSegment::LineSegment(LineSegment(point![4.0, 1.0], point![4.0, 4.0])),
            ],
        }
    }

    pub mod verify_gradient {
        use std::fmt::Debug;

        use approx::{assert_relative_eq, AbsDiffEq, RelativeEq};
        use num_traits::{real::Real, NumAssignOps};

        use crate::traits::{
            decompose::Decompose,
            gradient_type::{Gradient, GradientType},
        };

        pub fn verify_gradient<
            A: Clone + Debug + Decompose<F> + GradientType,
            F: Real + NumAssignOps,
        >(
            func: &impl Fn(A) -> F,
            grad: &impl Fn(A) -> Gradient<A>,
            epsilon: <Gradient<A> as AbsDiffEq>::Epsilon,
            x: A,
        ) where
            Gradient<A>: Debug + RelativeEq + Decompose<F>,
            <Gradient<A> as AbsDiffEq>::Epsilon: From<f32>,
        {
            let real_gradient = grad(x.clone());
            let numerical_gradient = numerical_grad(func, x);

            assert_relative_eq!(real_gradient, numerical_gradient, epsilon = epsilon);
        }

        fn numerical_grad<A: Clone + Decompose<F> + GradientType, F: Real + NumAssignOps>(
            func: &impl Fn(A) -> F,
            x: A,
        ) -> Gradient<A>
        where
            Gradient<A>: Decompose<F>,
        {
            let decomposed = (0..A::N)
                .map(|i| numerical_nth_derivative(func, i, x.clone()))
                .collect();

            Gradient::<A>::compose(decomposed)
        }

        fn numerical_nth_derivative<A: Decompose<F>, F: Real + NumAssignOps>(
            func: &impl Fn(A) -> F,
            n: usize,
            x: A,
        ) -> F {
            let eps = F::from(1e-4).unwrap();

            let middle = x.decompose();
            let above = {
                let mut above = middle.clone();
                above[n] += eps;

                A::compose(above)
            };
            let below = {
                let mut below = middle.clone();
                below[n] -= eps;

                A::compose(below)
            };

            let sample_above = func(above);
            let sample_below = func(below);

            let difference = sample_above - sample_below;
            let sample_distance = F::from(2.0).unwrap() * eps;

            difference / sample_distance
        }
    }
}
