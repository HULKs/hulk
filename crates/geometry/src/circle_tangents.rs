use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{line_segment::LineSegment, two_line_segments::TwoLineSegments};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct CircleTangents {
    pub inner: Option<TwoLineSegments>,
    pub outer: TwoLineSegments,
}

impl AbsDiffEq for CircleTangents {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.inner.is_some() == other.inner.is_some()
            && LineSegment::abs_diff_eq(&other.outer.0, &self.outer.0, epsilon)
            && LineSegment::abs_diff_eq(&other.outer.1, &self.outer.1, epsilon)
            && if self.inner.is_some() && other.inner.is_some() {
                LineSegment::abs_diff_eq(&other.inner.unwrap().0, &self.inner.unwrap().0, epsilon)
                    && LineSegment::abs_diff_eq(
                        &other.inner.unwrap().1,
                        &self.inner.unwrap().1,
                        epsilon,
                    )
            } else {
                true
            }
    }
}

impl RelativeEq for CircleTangents {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.inner.is_some() == other.inner.is_some()
            && LineSegment::relative_eq(&other.outer.0, &self.outer.0, epsilon, max_relative)
            && LineSegment::relative_eq(&other.outer.1, &self.outer.1, epsilon, max_relative)
            && if self.inner.is_some() && other.inner.is_some() {
                LineSegment::relative_eq(
                    &other.inner.unwrap().0,
                    &self.inner.unwrap().0,
                    epsilon,
                    max_relative,
                ) && LineSegment::relative_eq(
                    &other.inner.unwrap().1,
                    &self.inner.unwrap().1,
                    epsilon,
                    max_relative,
                )
            } else {
                true
            }
    }
}
