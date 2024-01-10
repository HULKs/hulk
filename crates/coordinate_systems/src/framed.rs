use std::{
    marker::PhantomData,
    ops::{Add, Sub},
};

use geometry::look_at::LookAt;

#[derive(Debug)]
pub struct Framed<Frame, Inner> {
    frame: PhantomData<Frame>,
    pub inner: Inner,
}

impl<Frame, Inner> Clone for Framed<Frame, Inner>
where
    Inner: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl<Frame, Inner> Copy for Framed<Frame, Inner> where Inner: Copy {}

impl<Frame, Inner> Framed<Frame, Inner> {
    pub fn new(inner: Inner) -> Self {
        Self {
            frame: PhantomData,
            inner,
        }
    }
}

impl<Frame, SelfInner, RightInner> Add<Framed<Frame, RightInner>> for Framed<Frame, SelfInner>
where
    SelfInner: Add<RightInner>,
{
    type Output = Framed<Frame, SelfInner::Output>;

    fn add(self, right: Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::new(self.inner + right.inner)
    }
}

impl<Frame, SelfInner, RightInner> Sub<Framed<Frame, RightInner>> for Framed<Frame, SelfInner>
where
    SelfInner: Sub<RightInner>,
{
    type Output = Framed<Frame, SelfInner::Output>;

    fn sub(self, right: Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::new(self.inner - right.inner)
    }
}

impl<Frame, Inner> LookAt<Framed<Frame, Inner>> for Framed<Frame, Inner>
where
    Inner: LookAt<Inner>,
{
    type Rotation = Framed<Frame, Inner::Rotation>;

    fn look_at(&self, target: &Self) -> Self::Rotation {
        Self::Rotation::new(self.inner.look_at(&target.inner))
    }
}
