use nalgebra::{allocator::Allocator, DefaultAllocator, Dim, Point2, Scalar, Vector2, U1};
use num_dual::{Derivative, DualNum};
use num_traits::Float;

use linear_algebra::Framed;

use crate::geometry::{angle::Angle, normalized_step::NormalizedStep, pose::PoseGradient};

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

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> ScaledGradient<T, F, D, PoseGradient<T>>
    for PoseGradient<Derivative<T, F, D, U1>>
where
    DefaultAllocator: Allocator<D>,
{
    fn scaled_gradient(self, scale: PoseGradient<T>) -> Derivative<T, F, D, U1> {
        let PoseGradient {
            position: position_gradient,
            orientation: orientation_gradient,
        } = self;
        let PoseGradient {
            position: position_derivative,
            orientation: orientation_derivative,
        } = scale;

        position_gradient.scaled_gradient(position_derivative)
            + orientation_gradient * orientation_derivative
    }
}

impl<T: DualNum<F>, F: Float + Scalar, D: Dim> ScaledGradient<T, F, D, NormalizedStep<T>>
    for NormalizedStep<Derivative<T, F, D, U1>>
where
    DefaultAllocator: Allocator<D>,
{
    fn scaled_gradient(self, scale: NormalizedStep<T>) -> Derivative<T, F, D, U1> {
        let NormalizedStep {
            forward: forward_gradient,
            left: left_gradient,
            turn: turn_gradient,
        } = self;
        let NormalizedStep {
            forward: forward_derivative,
            left: left_derivative,
            turn: turn_derivative,
        } = scale;

        forward_gradient * forward_derivative
            + left_gradient * left_derivative
            + turn_gradient * turn_derivative
    }
}
