use core::f32;

use approx::AbsDiffEq;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::circle::Circle;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
    Serialize,
)]
pub struct Ellipse<Frame> {
    pub center: Point2<Frame>,
    pub major_radius: f32,
    pub minor_radius: f32,
    pub angle: f32,
}

impl<Frame> Ellipse<Frame> {
    pub fn new(center: Point2<Frame>, major_radius: f32, minor_radius: f32, angle: f32) -> Self {
        Self {
            center,
            major_radius,
            minor_radius,
            angle,
        }
    }

    pub fn is_circle(&self) -> bool {
        self.major_radius
            .abs_diff_eq(&self.minor_radius, f32::EPSILON)
    }
}

impl<Frame> From<Circle<Frame>> for Ellipse<Frame> {
    fn from(circle: Circle<Frame>) -> Self {
        Self {
            center: circle.center,
            major_radius: circle.radius,
            minor_radius: circle.radius,
            angle: 0.0,
        }
    }
}

impl<Frame> TryInto<Circle<Frame>> for Ellipse<Frame> {
    type Error = ();

    fn try_into(self) -> Result<Circle<Frame>, Self::Error> {
        if self.is_circle() {
            Ok(Circle {
                center: self.center,
                radius: self.major_radius,
            })
        } else {
            Err(())
        }
    }
}
