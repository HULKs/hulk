use std::fmt::Debug;

use coordinate_systems::{Framed, IntoFramed};
use nalgebra::{point, Point2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{coordinate_systems::Pixel, line::Line};

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct Limb {
    pub pixel_polygon: Vec<Framed<Pixel, Point2<f32>>>,
}

pub fn is_above_limbs(
    pixel_position: Framed<Pixel, Point2<f32>>,
    projected_limbs: &[Limb],
) -> bool {
    projected_limbs.iter().all(|limb| {
        match limb.pixel_polygon.as_slice().windows(2).find(|points| {
            points[0].inner.x <= pixel_position.inner.x
                && points[1].inner.x >= pixel_position.inner.x
        }) {
            Some(points) => {
                if points[0].inner.x == points[1].inner.x {
                    return (pixel_position.inner.y)
                        < f32::min(points[0].inner.y, points[1].inner.y);
                }

                // since Y is pointing downwards, "is above" is actually !Line::is_above()
                !Line(points[0], points[1])
                    .is_above(point![pixel_position.inner.x, pixel_position.inner.y].framed())
            }
            None => true,
        }
    })
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct ProjectedLimbs {
    pub limbs: Vec<Limb>,
}
