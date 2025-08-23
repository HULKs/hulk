use nalgebra::{allocator::Allocator, DefaultAllocator, DimName, OPoint, OVector, Scalar};

use linear_algebra::Framed;
use types::step::Step;

use crate::geometry::{
    angle::Angle,
    orientation::Orientation,
    pose::{Pose, PoseGradient},
};

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
    Angle<T>, T
);
impl_gradient!(
    <T>;
    Orientation<T>, T
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
