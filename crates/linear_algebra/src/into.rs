use crate::{framed::Framed, transform::Transform};

/// Trait for converting a value into its framed representation.
///
/// This allows wrapping a value in a [`Framed`] type with a specified frame.
pub trait IntoFramed
where
    Self: Sized,
{
    /// Wraps `self` in a [`Framed`] with the given frame type.
    fn framed<Frame>(self) -> Framed<Frame, Self>;
}

impl<Inner> IntoFramed for Inner {
    fn framed<Frame>(self) -> Framed<Frame, Self> {
        Framed::wrap(self)
    }
}

/// Trait for converting a value into a framed transform.
///
/// This allows wrapping a value in a [`Transform`] type with specified source and target frames.
pub trait IntoTransform
where
    Self: Sized,
{
    /// Wraps `self` in a [`Transform`] with the given source `From` and target `To` frame types.
    fn framed_transform<From, To>(self) -> Transform<From, To, Self>;
}

impl<T> IntoTransform for T {
    fn framed_transform<From, To>(self) -> Transform<From, To, Self> {
        Transform::wrap(self)
    }
}
