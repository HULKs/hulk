use nalgebra::{allocator::Allocator, DefaultAllocator, Dim, Point2, Scalar, Vector2, U1};
use num_dual::{Derivative, DualNum};
use num_traits::Float;

use linear_algebra::Framed;
use types::step::Step;

use crate::{
    geometry::{angle::Angle, Pose},
    step_plan::{PlannedStep, PlannedStepGradient},
};

pub trait ScaledGradient<T: DualNum<F>, F, D: Dim, S>
where
    DefaultAllocator: Allocator<D>,
{
    fn scaled_gradient(self, scale: S) -> Derivative<T, F, D, U1>;
}

impl<Frame, SelfInner, OtherInner, T: DualNum<F>, F: Float + Scalar, D: Dim>
    ScaledGradient<T, F, D, Framed<Frame, OtherInner>> for Framed<Frame, SelfInner>
where
    DefaultAllocator: Allocator<D>,
    SelfInner: ScaledGradient<T, F, D, OtherInner>,
{
    fn scaled_gradient(self, scale: Framed<Frame, OtherInner>) -> Derivative<T, F, D, U1> {
        self.inner.scaled_gradient(scale.inner)
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> ScaledGradient<T, F, D, Angle<T>>
    for Angle<Derivative<T, F, D, U1>>
where
    DefaultAllocator: Allocator<D>,
{
    fn scaled_gradient(self, scale: Angle<T>) -> Derivative<T, F, D, U1> {
        self.0 * scale.0
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> ScaledGradient<T, F, D, Vector2<T>>
    for Vector2<Derivative<T, F, D, U1>>
where
    DefaultAllocator: Allocator<D>,
{
    fn scaled_gradient(self, scale: Vector2<T>) -> Derivative<T, F, D, U1> {
        let [[x_gradient, y_gradient]] = self.data.0;
        let [[x_derivative, y_derivative]] = scale.data.0;

        x_gradient * x_derivative + y_gradient * y_derivative
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> ScaledGradient<T, F, D, Point2<T>>
    for Point2<Derivative<T, F, D, U1>>
where
    DefaultAllocator: Allocator<D>,
{
    fn scaled_gradient(self, scale: Point2<T>) -> Derivative<T, F, D, U1> {
        let [[x_gradient, y_gradient]] = self.coords.data.0;
        let [[x_derivative, y_derivative]] = scale.coords.data.0;

        x_gradient * x_derivative + y_gradient * y_derivative
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> ScaledGradient<T, F, D, Pose<T>>
    for Pose<Derivative<T, F, D, U1>>
where
    DefaultAllocator: Allocator<D>,
{
    fn scaled_gradient(self, scale: Pose<T>) -> Derivative<T, F, D, U1> {
        let Pose {
            position: position_gradient,
            orientation: orientation_gradient,
        } = self;
        let Pose {
            position: position_derivative,
            orientation: orientation_derivative,
        } = scale;

        position_gradient.scaled_gradient(position_derivative)
            + orientation_gradient * orientation_derivative
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> ScaledGradient<T, F, D, Step<T>>
    for Step<Derivative<T, F, D, U1>>
where
    DefaultAllocator: Allocator<D>,
{
    fn scaled_gradient(self, scale: Step<T>) -> Derivative<T, F, D, U1> {
        let Step {
            forward: forward_gradient,
            left: left_gradient,
            turn: turn_gradient,
        } = self;
        let Step {
            forward: forward_derivative,
            left: left_derivative,
            turn: turn_derivative,
        } = scale;

        forward_gradient * forward_derivative
            + left_gradient * left_derivative
            + turn_gradient * turn_derivative
    }
}

// impl<T: DualNum<F>, F: Float + Scalar, D: Dim> ScaledGradient<T, F, D, PlannedStep<T>>
//     for PlannedStep<Derivative<T, F, D, U1>>
// where
//     DefaultAllocator: Allocator<D>,
// {
//     fn scaled_gradient(self, scale: PlannedStep<T>) -> Derivative<T, F, D, U1> {
//         let PlannedStep {
//             pose: pose_gradient,
//             step: step_gradient,
//             support_foot: _,
//         } = self;
//         let PlannedStep {
//             pose: pose_derivative,
//             step: step_derivative,
//             support_foot: _,
//         } = scale;

//         pose_gradient.scaled_gradient(pose_derivative)
//             + step_gradient.scaled_gradient(step_derivative)
//     }
// }

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> ScaledGradient<T, F, D, PlannedStepGradient<T>>
    for PlannedStep<Derivative<T, F, D, U1>>
where
    DefaultAllocator: Allocator<D>,
{
    fn scaled_gradient(self, scale: PlannedStepGradient<T>) -> Derivative<T, F, D, U1> {
        let PlannedStep {
            pose: pose_gradient,
            step: step_gradient,
            ..
        } = self;
        let PlannedStepGradient {
            pose: pose_derivative,
            step: step_derivative,
        } = scale;

        pose_gradient.scaled_gradient(pose_derivative)
            + step_gradient.step.scaled_gradient(step_derivative)
    }
}
