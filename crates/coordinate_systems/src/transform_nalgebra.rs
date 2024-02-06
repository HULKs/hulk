use nalgebra::{AbstractRotation, Isometry, SimdRealField, UnitComplex};

use crate::Transform;

// Isometry

impl<From, To, Type, Rotation, const DIMENSION: usize>
    Transform<From, To, Isometry<Type, Rotation, DIMENSION>>
where
    Type::Element: SimdRealField,
    Type: SimdRealField,
    Rotation: AbstractRotation<Type, DIMENSION>,
{
    pub fn inverse(&self) -> Transform<To, From, Isometry<Type, Rotation, DIMENSION>> {
        Transform::<To, From, _>::new(self.inner.inverse())
    }
}

// UnitComplex

impl<From, To, Type> Transform<From, To, UnitComplex<Type>>
where
    Type::Element: SimdRealField,
    Type: SimdRealField,
{
    pub fn inverse(&self) -> Transform<To, From, UnitComplex<Type>> {
        Transform::<To, From, _>::new(self.inner.inverse())
    }
}
