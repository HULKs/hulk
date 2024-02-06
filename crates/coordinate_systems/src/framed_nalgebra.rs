use nalgebra::{ArrayStorage, Const, Matrix, Point, SVector, Scalar};

use crate::Framed;

// Vectors

impl<Frame, const DIMENSION: usize> Framed<Frame, SVector<f32, DIMENSION>> {
    pub fn zeros() -> Self {
        Self::new(SVector::zeros())
    }

    pub fn normalize(&self) -> Self {
        Self::new(self.inner.normalize())
    }

    pub fn cap_magnitude(&self, max: f32) -> Self {
        Self::new(self.inner.cap_magnitude(max))
    }

    pub fn transpose(
        &self,
    ) -> Framed<Frame, Matrix<f32, Const<1>, Const<DIMENSION>, ArrayStorage<f32, 1, DIMENSION>>>
    {
        Framed::new(self.inner.transpose())
    }

    pub fn norm(&self) -> f32 {
        self.inner.norm()
    }

    pub fn norm_squared(&self) -> f32 {
        self.inner.norm_squared()
    }

    pub fn dot(&self, rhs: &Self) -> f32 {
        self.inner.dot(&rhs.inner)
    }

    pub fn angle(&self, rhs: &Self) -> f32 {
        self.inner.angle(&rhs.inner)
    }
}

impl<Frame, From, const DIMENSION: usize> Framed<Frame, SVector<From, DIMENSION>>
where
    From: Scalar,
{
    pub fn map<F, To>(&self, f: F) -> Framed<Frame, SVector<To, DIMENSION>>
    where
        To: Scalar,
        F: FnMut(From) -> To,
    {
        Framed::new(self.inner.map(f))
    }
}

impl<Frame> Framed<Frame, SVector<f32, 2>> {
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    pub fn y(&self) -> f32 {
        self.inner.y
    }
}

impl<Frame> Framed<Frame, SVector<f32, 3>> {
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    pub fn y(&self) -> f32 {
        self.inner.y
    }

    pub fn z(&self) -> f32 {
        self.inner.z
    }
}

// Points

pub fn distance<Frame, const DIMENSION: usize>(
    p1: &Framed<Frame, Point<f32, DIMENSION>>,
    p2: &Framed<Frame, Point<f32, DIMENSION>>,
) -> f32 {
    nalgebra::distance(&p1.inner, &p2.inner)
}

pub fn distance_squared<Frame, const DIMENSION: usize>(
    p1: &Framed<Frame, Point<f32, DIMENSION>>,
    p2: &Framed<Frame, Point<f32, DIMENSION>>,
) -> f32 {
    nalgebra::distance_squared(&p1.inner, &p2.inner)
}

pub fn center<Frame, const DIMENSION: usize>(
    p1: &Framed<Frame, Point<f32, DIMENSION>>,
    p2: &Framed<Frame, Point<f32, DIMENSION>>,
) -> Framed<Frame, Point<f32, DIMENSION>> {
    Framed::new(nalgebra::center(&p1.inner, &p2.inner))
}

impl<Frame, const DIMENSION: usize> Framed<Frame, Point<f32, DIMENSION>> {
    pub fn origin() -> Self {
        Self::new(Point::origin())
    }

    pub fn coords(&self) -> Framed<Frame, SVector<f32, DIMENSION>> {
        Framed::new(self.inner.coords)
    }
}

impl<Frame, From, const DIMENSION: usize> Framed<Frame, Point<From, DIMENSION>>
where
    From: Scalar,
{
    pub fn map<F, To>(&self, f: F) -> Framed<Frame, Point<To, DIMENSION>>
    where
        To: Scalar,
        F: FnMut(From) -> To,
    {
        Framed::new(self.inner.map(f))
    }
}

impl<Frame> Framed<Frame, Point<f32, 2>> {
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    pub fn y(&self) -> f32 {
        self.inner.y
    }
}

impl<Frame> Framed<Frame, Point<f32, 3>> {
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    pub fn y(&self) -> f32 {
        self.inner.y
    }

    pub fn z(&self) -> f32 {
        self.inner.z
    }
}
