mod classify_projection;
mod end_points;
mod length;
mod path_progress;
mod project;
mod scaled_gradient;
mod tangent;
mod wrap_dual;

pub use classify_projection::{ArcProjectionKind, ClassifyProjection};
pub use end_points::EndPoints;
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

    use crate::geometry::{pose::Pose, pose::PoseGradient};

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
        <T: Scalar>;
        Pose<T>, PoseGradient<T>
    );
    impl_gradient!(
        <Frame, Inner: GradientType>;
        Framed<Frame, Inner>, Framed<Frame, Gradient<Inner>>
    );
}
