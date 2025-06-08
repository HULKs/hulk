use std::fmt::Debug;

use nalgebra::{allocator::Allocator, DefaultAllocator, DimName, Matrix, OPoint, Owned, Scalar};

use linear_algebra::{point, vector, Framed};
use types::step::Step;

use crate::geometry::{angle::Angle, pose::PoseGradient, Pose};

pub trait Decompose<F> {
    const N: usize;

    fn decompose(self) -> Vec<F>; // Ideally this would return [F; Self::N], but Rust doesn't (yet) support it

    fn compose(decomposed: Vec<F>) -> Self;
}

impl<F: Debug> Decompose<F> for Step<F> {
    const N: usize = 3;

    fn decompose(self) -> Vec<F> {
        let Self {
            forward,
            left,
            turn,
        } = self;

        vec![forward, left, turn]
    }

    fn compose(decomposed: Vec<F>) -> Self {
        let [forward, left, turn] = decomposed.try_into().unwrap();

        Self {
            forward,
            left,
            turn,
        }
    }
}

impl<F: Scalar> Decompose<F> for Pose<F> {
    const N: usize = 3;

    fn decompose(self) -> Vec<F> {
        let Self {
            position,
            orientation,
        } = self;
        let [[x, y]] = position.inner.coords.data.0;

        vec![x, y, orientation.into_inner()]
    }

    fn compose(decomposed: Vec<F>) -> Self {
        let [x, y, orientation] = decomposed.try_into().unwrap();

        let position = point![x, y];
        let orientation = Angle(orientation);

        Self {
            position,
            orientation,
        }
    }
}

impl<F: Scalar> Decompose<F> for PoseGradient<F> {
    const N: usize = 3;

    fn decompose(self) -> Vec<F> {
        let Self {
            position,
            orientation,
        } = self;
        let [[x, y]] = position.inner.data.0;

        vec![x, y, orientation]
    }

    fn compose(decomposed: Vec<F>) -> Self {
        let [x, y, orientation] = decomposed.try_into().unwrap();

        let position = vector![x, y];

        Self {
            position,
            orientation,
        }
    }
}

impl<Frame, Inner: Decompose<F>, F> Decompose<F> for Framed<Frame, Inner> {
    const N: usize = Inner::N;

    fn decompose(self) -> Vec<F> {
        self.inner.decompose()
    }

    fn compose(decomposed: Vec<F>) -> Self {
        Self::wrap(Inner::compose(decomposed))
    }
}

impl<F: Scalar, D: DimName> Decompose<F> for OPoint<F, D>
where
    DefaultAllocator: Allocator<D>,
{
    const N: usize = D::USIZE;

    fn decompose(self) -> Vec<F> {
        self.coords.decompose()
    }

    fn compose(decomposed: Vec<F>) -> Self {
        Self::from_slice(&decomposed)
    }
}

impl<F: Scalar, R: DimName, C: DimName> Decompose<F> for Matrix<F, R, C, Owned<F, R, C>>
where
    DefaultAllocator: Allocator<R, C>,
{
    const N: usize = todo!();

    fn decompose(self) -> Vec<F> {
        self.as_slice().to_vec()
    }

    fn compose(decomposed: Vec<F>) -> Self {
        Self::from_column_slice(&decomposed)
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::{matrix, OMatrix, U2};

    use crate::test_utils::decompose::Decompose;

    #[test]
    fn decompose_matrix() {
        type M = OMatrix<f32, U2, U2>;

        let mat: M = matrix![
            1., 2.;
            3., 4.
        ];

        assert_eq!(mat, M::compose(mat.decompose()))
    }
}
