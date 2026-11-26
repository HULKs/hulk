use crate::{framed::Framed, transform::Transform};

/// Adds a coordinate frame label to a value.
pub trait IntoFramed
where
    Self: Sized,
{
    /// Labels `self` as already expressed in `Frame`, without changing its coordinates.
    fn framed<Frame>(self) -> Framed<Frame, Self>;
}

impl<Inner> IntoFramed for Inner {
    fn framed<Frame>(self) -> Framed<Frame, Self> {
        Framed::wrap(self)
    }
}

/// Adds source and destination labels to a coordinate mapping.
pub trait IntoTransform
where
    Self: Sized,
{
    /// Labels `self` as an existing mapping from `From` to `To`, without changing it.
    fn framed_transform<From, To>(self) -> Transform<From, To, Self>;
}

impl<T> IntoTransform for T {
    fn framed_transform<From, To>(self) -> Transform<From, To, Self> {
        Transform::wrap(self)
    }
}
