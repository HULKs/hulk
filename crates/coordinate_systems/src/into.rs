use crate::{framed::Framed, transform::Transform};

pub trait IntoFramed
where
    Self: Sized,
{
    fn framed<Frame>(self) -> Framed<Frame, Self>;
}

impl<Inner> IntoFramed for Inner {
    fn framed<Frame>(self) -> Framed<Frame, Self> {
        Framed::new(self)
    }
}

pub trait IntoTransform
where
    Self: Sized,
{
    fn framed_transform<From, To>(self) -> Transform<From, To, Self>;
}

impl<T> IntoTransform for T {
    fn framed_transform<From, To>(self) -> Transform<From, To, Self> {
        Transform::new(self)
    }
}
