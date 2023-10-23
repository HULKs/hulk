use std::{
    marker::PhantomData,
    ops::{Add, Sub},
};

//#[derive(Clone, Copy, Debug)]
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

impl<Frame, SelfInner, RhsInner> Add<Framed<Frame, RhsInner>> for Framed<Frame, SelfInner>
where
    SelfInner: Add<RhsInner>,
{
    type Output = Framed<Frame, SelfInner::Output>;

    fn add(self, rhs: Framed<Frame, RhsInner>) -> Self::Output {
        Self::Output::new(self.inner + rhs.inner)
    }
}

impl<Frame, SelfInner, RhsInner> Sub<Framed<Frame, RhsInner>> for Framed<Frame, SelfInner>
where
    SelfInner: Sub<RhsInner>,
{
    type Output = Framed<Frame, SelfInner::Output>;

    fn sub(self, rhs: Framed<Frame, RhsInner>) -> Self::Output {
        Self::Output::new(self.inner - rhs.inner)
    }
}
