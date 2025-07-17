use nalgebra::{
    allocator::Allocator, point, vector, DefaultAllocator, Dim, Point2, Scalar, Vector2, U1,
};
use num_dual::{Derivative, DualNum, DualVec};
use num_traits::Float;

use linear_algebra::{Framed, IntoFramed};
use types::step::Step;

use crate::geometry::{
    angle::Angle,
    normalized_step::NormalizedStep,
    orientation::Orientation,
    pose::{Pose, PoseGradient},
};

pub trait WrapDual<Real, Gradient> {
    fn wrap_dual(real: Real) -> Self;
    fn unwrap_dual(self) -> (Real, Gradient);
}

impl<Frame, SelfInner: WrapDual<Real, Gradient>, Real, Gradient>
    WrapDual<Framed<Frame, Real>, Framed<Frame, Gradient>> for Framed<Frame, SelfInner>
{
    fn wrap_dual(real: Framed<Frame, Real>) -> Self {
        SelfInner::wrap_dual(real.inner).framed()
    }

    fn unwrap_dual(self) -> (Framed<Frame, Real>, Framed<Frame, Gradient>) {
        let (real, gradient) = self.inner.unwrap_dual();

        (real.framed(), gradient.framed())
    }
}

impl<T: DualNum<F>, F, D: Dim> WrapDual<T, Derivative<T, F, D, U1>> for DualVec<T, F, D>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(real: T) -> DualVec<T, F, D> {
        DualVec::from_re(real)
    }

    fn unwrap_dual(self) -> (T, Derivative<T, F, D, U1>) {
        (self.re, self.eps)
    }
}

impl<T: DualNum<F>, F, D: Dim> WrapDual<Angle<T>, Derivative<T, F, D, U1>>
    for Angle<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(real: Angle<T>) -> Angle<DualVec<T, F, D>> {
        Angle(DualVec::from_re(real.0))
    }

    fn unwrap_dual(self) -> (Angle<T>, Derivative<T, F, D, U1>) {
        let (re, eps) = self.0.unwrap_dual();

        (Angle(re), eps)
    }
}

impl<T: DualNum<F>, F, D: Dim> WrapDual<Orientation<T>, Derivative<T, F, D, U1>>
    for Orientation<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(real: Orientation<T>) -> Orientation<DualVec<T, F, D>> {
        Orientation(DualVec::from_re(real.0))
    }

    fn unwrap_dual(self) -> (Orientation<T>, Derivative<T, F, D, U1>) {
        let (re, eps) = self.0.unwrap_dual();

        (Orientation(re), eps)
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> WrapDual<Point2<T>, Vector2<Derivative<T, F, D, U1>>>
    for Point2<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(real: Point2<T>) -> Point2<DualVec<T, F, D>> {
        real.map(DualVec::from_re)
    }

    fn unwrap_dual(self) -> (Point2<T>, Vector2<Derivative<T, F, D, U1>>) {
        let [[x, y]] = self.coords.data.0;

        let (x, d_x) = x.unwrap_dual();
        let (y, d_y) = y.unwrap_dual();

        (point![x, y], vector![d_x, d_y])
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim>
    WrapDual<Vector2<T>, Vector2<Derivative<T, F, D, U1>>> for Vector2<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(real: Vector2<T>) -> Vector2<DualVec<T, F, D>> {
        real.map(DualVec::from_re)
    }

    fn unwrap_dual(self) -> (Vector2<T>, Vector2<Derivative<T, F, D, U1>>) {
        let [[x, y]] = self.data.0;

        let (x, d_x) = x.unwrap_dual();
        let (y, d_y) = y.unwrap_dual();

        (vector![x, y], vector![d_x, d_y])
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim>
    WrapDual<Pose<T>, PoseGradient<Derivative<T, F, D, U1>>> for Pose<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(real: Pose<T>) -> Pose<DualVec<T, F, D>> {
        Pose {
            position: WrapDual::wrap_dual(real.position),
            orientation: WrapDual::wrap_dual(real.orientation),
        }
    }

    fn unwrap_dual(self) -> (Pose<T>, PoseGradient<Derivative<T, F, D, U1>>) {
        let Pose {
            position,
            orientation,
        } = self;

        let (position, d_position) = position.unwrap_dual();
        let (orientation, d_orientation) = orientation.unwrap_dual();

        (
            Pose {
                position,
                orientation,
            },
            PoseGradient {
                position: d_position,
                orientation: d_orientation,
            },
        )
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> WrapDual<Step<T>, Step<Derivative<T, F, D, U1>>>
    for Step<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(real: Step<T>) -> Step<DualVec<T, F, D>> {
        Step {
            forward: WrapDual::wrap_dual(real.forward),
            left: WrapDual::wrap_dual(real.left),
            turn: WrapDual::wrap_dual(real.turn),
        }
    }

    fn unwrap_dual(self) -> (Step<T>, Step<Derivative<T, F, D, U1>>) {
        let Self {
            forward,
            left,
            turn,
        } = self;

        let (forward, d_forward) = forward.unwrap_dual();
        let (left, d_left) = left.unwrap_dual();
        let (turn, d_turn) = turn.unwrap_dual();

        (
            Step {
                forward,
                left,
                turn,
            },
            Step {
                forward: d_forward,
                left: d_left,
                turn: d_turn,
            },
        )
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim>
    WrapDual<NormalizedStep<T>, NormalizedStep<Derivative<T, F, D, U1>>>
    for NormalizedStep<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(real: NormalizedStep<T>) -> NormalizedStep<DualVec<T, F, D>> {
        NormalizedStep {
            forward: WrapDual::wrap_dual(real.forward),
            left: WrapDual::wrap_dual(real.left),
            turn: WrapDual::wrap_dual(real.turn),
        }
    }

    fn unwrap_dual(self) -> (NormalizedStep<T>, NormalizedStep<Derivative<T, F, D, U1>>) {
        let Self {
            forward,
            left,
            turn,
        } = self;

        let (forward, d_forward) = forward.unwrap_dual();
        let (left, d_left) = left.unwrap_dual();
        let (turn, d_turn) = turn.unwrap_dual();

        (
            NormalizedStep {
                forward,
                left,
                turn,
            },
            NormalizedStep {
                forward: d_forward,
                left: d_left,
                turn: d_turn,
            },
        )
    }
}
