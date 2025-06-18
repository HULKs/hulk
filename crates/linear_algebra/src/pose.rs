use nalgebra::SimdRealField;

use crate::{Framed, Isometry2, Isometry3, Orientation2, Orientation3, Point2, Point3};

pub type Pose2<Frame, T = f32> = Framed<Frame, nalgebra::Isometry2<T>>;
pub type Pose3<Frame, T = f32> = Framed<Frame, nalgebra::Isometry3<T>>;

// 2 Dimension

impl<Frame, T> Pose2<Frame, T>
where
    T: SimdRealField + Copy,
    T::Element: SimdRealField,
{
    /// Create a new pose from a position and angle.
    ///
    /// # Example
    /// ```
    /// use linear_algebra::{point, Point2, Pose2};
    ///
    /// struct Robot;
    /// let pose = Pose2::<Robot>::new(point![1.0, 2.0], 0.5);
    /// ```
    pub fn new(position: Point2<Frame, T>, angle: T) -> Self {
        Self::wrap(nalgebra::Isometry2::new(position.inner.coords, angle))
    }

    /// Create a new pose from a position and orientation.
    ///
    /// # Example
    /// ```
    /// use linear_algebra::{point, Orientation2, Pose2};
    ///
    /// struct Robot;
    /// let position = point![1.0, 2.0];
    /// let orientation = Orientation2::<Robot>::new(0.5);
    /// let pose = Pose2::<Robot>::from_parts(position, orientation);
    /// ```
    pub fn from_parts(position: Point2<Frame, T>, orientation: Orientation2<Frame, T>) -> Self {
        Self::wrap(nalgebra::Isometry2::from_parts(
            position.inner.into(),
            orientation.inner,
        ))
    }

    /// Returns a new pose with zero translation and zero orientation.
    pub fn zero() -> Self {
        Default::default()
    }

    /// Returns the pose as an isometry transform, labeling the Pose frame as `From`.
    ///
    /// # Example
    /// ```
    /// use linear_algebra::{point, Isometry2, Pose2};
    ///
    /// struct World;
    /// let robot = Pose2::<World>::new(point![1.0, 2.0], 0.5);
    ///
    /// struct Robot;
    /// let robot_to_world: Isometry2<Robot, World> = robot.as_transform();
    /// ```
    pub fn as_transform<From>(&self) -> Isometry2<From, Frame, T> {
        Isometry2::wrap(self.inner)
    }

    pub fn position(&self) -> Point2<Frame, T> {
        Point2::wrap(self.inner.translation.vector.into())
    }

    pub fn orientation(&self) -> Orientation2<Frame, T>
    where
        T: Copy,
    {
        Orientation2::wrap(self.inner.rotation)
    }

    pub fn angle(&self) -> T {
        self.inner.rotation.angle()
    }
}

impl<Frame, T> From<Point2<Frame, T>> for Pose2<Frame, T>
where
    T: SimdRealField,
    T::Element: SimdRealField,
{
    fn from(value: Point2<Frame, T>) -> Self {
        Self::wrap(nalgebra::Isometry2::from(value.inner))
    }
}

// 3 Dimension

impl<Frame, T> Pose3<Frame, T>
where
    T: SimdRealField + Copy,
    T::Element: SimdRealField,
{
    pub fn from_parts(position: Point3<Frame, T>, orientation: Orientation3<Frame, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::from_parts(
            position.inner.into(),
            orientation.inner,
        ))
    }

    /// Returns the pose as an isometry transform, labeling the Pose frame as `From`.
    ///
    /// # Example
    /// ```
    /// use linear_algebra::{point, Isometry3, Orientation3, Pose3};
    ///
    /// struct World;
    /// let robot = Pose3::<World>::from_parts(point![1.0, 2.0, 0.0], Orientation3::default());
    ///
    /// struct Robot;
    /// let robot_to_world: Isometry3<Robot, World> = robot.as_transform();
    /// ```
    pub fn as_transform<From>(&self) -> Isometry3<From, Frame, T> {
        Isometry3::wrap(self.inner)
    }

    pub fn position(&self) -> Point3<Frame, T> {
        Point3::wrap(self.inner.translation.vector.into())
    }

    pub fn orientation(&self) -> Orientation3<Frame, T>
    where
        T: Copy,
    {
        Orientation3::wrap(self.inner.rotation)
    }
}

impl<Frame, T> From<Point3<Frame, T>> for Pose3<Frame, T>
where
    T: SimdRealField,
    T::Element: SimdRealField,
{
    fn from(value: Point3<Frame, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::from(value.inner))
    }
}
