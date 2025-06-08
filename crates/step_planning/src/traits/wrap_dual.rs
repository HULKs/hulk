use nalgebra::{
    allocator::Allocator, point, vector, DefaultAllocator, Dim, Point2, Scalar, Vector2, U1,
};
use num_dual::{Derivative, DualNum, DualVec};
use num_traits::Float;

use linear_algebra::{Framed, IntoFramed};
use types::step::{Step, StepAndSupportFoot};

use crate::{
    geometry::{
        angle::Angle,
        pose::{PoseAndSupportFoot, PoseGradient},
        Pose,
    },
    step_plan::{PlannedStep, PlannedStepGradient},
};

pub trait WrapDual<Dual> {
    fn wrap_dual(self) -> Dual;
}

pub trait UnwrapDual<Real, Gradient> {
    fn unwrap_dual(self) -> (Real, Gradient);
}

impl<Frame, SelfInner: WrapDual<OtherInner>, OtherInner> WrapDual<Framed<Frame, OtherInner>>
    for Framed<Frame, SelfInner>
{
    fn wrap_dual(self) -> Framed<Frame, OtherInner> {
        self.inner.wrap_dual().framed()
    }
}

impl<Frame, SelfInner: UnwrapDual<Real, Gradient>, Real, Gradient>
    UnwrapDual<Framed<Frame, Real>, Framed<Frame, Gradient>> for Framed<Frame, SelfInner>
{
    fn unwrap_dual(self) -> (Framed<Frame, Real>, Framed<Frame, Gradient>) {
        let (real, gradient) = self.inner.unwrap_dual();

        (real.framed(), gradient.framed())
    }
}

impl<T: DualNum<F>, F, D: Dim> WrapDual<DualVec<T, F, D>> for T
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(self) -> DualVec<T, F, D> {
        DualVec::from_re(self)
    }
}

impl<T: DualNum<F>, F, D: Dim> UnwrapDual<T, Derivative<T, F, D, U1>> for DualVec<T, F, D>
where
    DefaultAllocator: Allocator<D>,
{
    fn unwrap_dual(self) -> (T, Derivative<T, F, D, U1>) {
        (self.re, self.eps)
    }
}

impl<T: DualNum<F>, F, D: Dim> WrapDual<Angle<DualVec<T, F, D>>> for Angle<T>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(self) -> Angle<DualVec<T, F, D>> {
        Angle::new(DualVec::from_re(self.0))
    }
}

impl<T: DualNum<F>, F, D: Dim> UnwrapDual<Angle<T>, Derivative<T, F, D, U1>>
    for Angle<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn unwrap_dual(self) -> (Angle<T>, Derivative<T, F, D, U1>) {
        let (re, eps) = self.0.unwrap_dual();

        (Angle::new(re), eps)
    }
}

impl<T: DualNum<F> + Scalar, F: Float + Scalar, D: Dim> WrapDual<Point2<DualVec<T, F, D>>>
    for Point2<T>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(self) -> Point2<DualVec<T, F, D>> {
        self.map(DualVec::from_re)
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim>
    UnwrapDual<Point2<T>, Vector2<Derivative<T, F, D, U1>>> for Point2<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn unwrap_dual(self) -> (Point2<T>, Vector2<Derivative<T, F, D, U1>>) {
        let [[x, y]] = self.coords.data.0;

        let (x, d_x) = x.unwrap_dual();
        let (y, d_y) = y.unwrap_dual();

        (point![x, y], vector![d_x, d_y])
    }
}

impl<T: DualNum<F> + Scalar, F: Float + Scalar, D: Dim> WrapDual<Vector2<DualVec<T, F, D>>>
    for Vector2<T>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(self) -> Vector2<DualVec<T, F, D>> {
        self.map(DualVec::from_re)
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim>
    UnwrapDual<Vector2<T>, Vector2<Derivative<T, F, D, U1>>> for Vector2<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn unwrap_dual(self) -> (Vector2<T>, Vector2<Derivative<T, F, D, U1>>) {
        let [[x, y]] = self.data.0;

        let (x, d_x) = x.unwrap_dual();
        let (y, d_y) = y.unwrap_dual();

        (vector![x, y], vector![d_x, d_y])
    }
}

impl<T: DualNum<F> + Scalar, F: Float + Scalar, D: Dim> WrapDual<Pose<DualVec<T, F, D>>> for Pose<T>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(self) -> Pose<DualVec<T, F, D>> {
        Pose {
            position: self.position.wrap_dual(),
            orientation: self.orientation.wrap_dual(),
        }
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim>
    UnwrapDual<Pose<T>, PoseGradient<Derivative<T, F, D, U1>>> for Pose<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
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

impl<T: DualNum<F> + Scalar, F: Float + Scalar, D: Dim> WrapDual<Step<DualVec<T, F, D>>> for Step<T>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(self) -> Step<DualVec<T, F, D>> {
        Step {
            forward: self.forward.wrap_dual(),
            left: self.left.wrap_dual(),
            turn: self.turn.wrap_dual(),
        }
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> UnwrapDual<Step<T>, Step<Derivative<T, F, D, U1>>>
    for Step<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
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

impl<T: DualNum<F> + Scalar, F: Float + Scalar, D: Dim>
    WrapDual<StepAndSupportFoot<DualVec<T, F, D>>> for StepAndSupportFoot<T>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(self) -> StepAndSupportFoot<DualVec<T, F, D>> {
        StepAndSupportFoot {
            step: self.step.wrap_dual(),
            support_foot: self.support_foot,
        }
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim>
    UnwrapDual<StepAndSupportFoot<T>, Step<Derivative<T, F, D, U1>>>
    for StepAndSupportFoot<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn unwrap_dual(self) -> (StepAndSupportFoot<T>, Step<Derivative<T, F, D, U1>>) {
        let Self { step, support_foot } = self;

        let (step, d_step) = step.unwrap_dual();

        (StepAndSupportFoot { step, support_foot }, d_step)
    }
}

impl<T: DualNum<F> + Scalar, F: Float + Scalar, D: Dim>
    WrapDual<PoseAndSupportFoot<DualVec<T, F, D>>> for PoseAndSupportFoot<T>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(self) -> PoseAndSupportFoot<DualVec<T, F, D>> {
        PoseAndSupportFoot {
            pose: self.pose.wrap_dual(),
            support_foot: self.support_foot,
        }
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim>
    UnwrapDual<PoseAndSupportFoot<T>, PoseGradient<Derivative<T, F, D, U1>>>
    for PoseAndSupportFoot<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn unwrap_dual(self) -> (PoseAndSupportFoot<T>, PoseGradient<Derivative<T, F, D, U1>>) {
        let Self { pose, support_foot } = self;

        let (pose, d_pose) = pose.unwrap_dual();

        (PoseAndSupportFoot { pose, support_foot }, d_pose)
    }
}

impl<T: DualNum<F> + Scalar, F: Float + Scalar, D: Dim> WrapDual<PlannedStep<DualVec<T, F, D>>>
    for PlannedStep<T>
where
    DefaultAllocator: Allocator<D>,
{
    fn wrap_dual(self) -> PlannedStep<DualVec<T, F, D>> {
        PlannedStep {
            pose: self.pose.wrap_dual(),
            step: self.step.wrap_dual(),
        }
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim>
    UnwrapDual<PlannedStep<T>, PlannedStepGradient<Derivative<T, F, D, U1>>>
    for PlannedStep<DualVec<T, F, D>>
where
    DefaultAllocator: Allocator<D>,
{
    fn unwrap_dual(self) -> (PlannedStep<T>, PlannedStepGradient<Derivative<T, F, D, U1>>) {
        let Self { pose, step } = self;

        let (pose, d_pose) = pose.unwrap_dual();
        let (step, d_step) = step.unwrap_dual();

        (
            PlannedStep { pose, step },
            PlannedStepGradient {
                pose: d_pose,
                step: d_step,
            },
        )
    }
}
