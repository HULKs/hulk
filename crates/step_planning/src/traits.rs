mod classify_projection;
mod length;
mod path_progress;
mod project;
mod scaled_gradient;
mod tangent;
mod wrap_dual;

pub use classify_projection::{ArcProjectionKind, ClassifyProjection};
pub use length::Length;
pub use path_progress::PathProgress;
pub use project::Project;
pub use scaled_gradient::ScaledGradient;
pub use tangent::Tangent;
pub use wrap_dual::{UnwrapDual, WrapDual};

pub mod gradient_type {
    use linear_algebra::Framed;
    use nalgebra::{allocator::Allocator, DefaultAllocator, DimName, OPoint, OVector, Scalar};
    use types::step::Step;

    pub trait GradientType {
        type Gradient;
    }

    pub type Gradient<T> = <T as GradientType>::Gradient;

    macro_rules! impl_gradient {
        ($a:ident, $b:ident) => {
        impl GradientType for $a {
                type Gradient = $b;
            }
        };
        (
            <$(
                $t:ident $(: $bound:ident $(+ $bound2:ident)* )?
            ),+> ;
            $a:ty, $b:ty
        ) => {
            impl<$($t $(: $bound $(+ $bound2)* )?),*> GradientType for $a {
                type Gradient = $b;
            }
        };
        (
            <$(
                $t:ident $(: $bound:ident $(+ $bound2:ident)* )?
            ),+> ;
            where $($where_type:ident : $where_bound:path),+ ;
            $a:ty, $b:ty
        ) => {
            impl<$($t $(: $bound $(+ $bound2)* )?),*> GradientType for $a
            where
                $($where_type: $where_bound)+
            {
                type Gradient = $b;
            }
        };
    }

    impl_gradient!(f32, f32);
    impl_gradient!(
        <T: Scalar, D: DimName>;
        where DefaultAllocator: Allocator<D>;
        OPoint<T, D>, OVector<T, D>
    );
    impl_gradient!(
        <T>;
        Step<T>, Step<T>
    );
    impl_gradient!(
        <Frame, Inner: GradientType>;
        Framed<Frame, Inner>, Framed<Frame, Gradient<Inner>>
    );
}

#[cfg(test)]
pub mod decompose {
    use std::fmt::Debug;

    use linear_algebra::Framed;
    use nalgebra::{
        allocator::Allocator, DefaultAllocator, DimName, Matrix, OPoint, Owned, Scalar,
    };
    use types::step::Step;

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

        use crate::traits::decompose::Decompose;

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
}
