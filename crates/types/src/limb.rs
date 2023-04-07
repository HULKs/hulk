use std::fmt::Debug;

use nalgebra::{point, Point2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::Line;

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct Limb {
    pub pixel_polygon: Vec<Point2<f32>>,
}

pub fn is_above_limbs(pixel_position: Point2<f32>, projected_limbs: &[Limb]) -> bool {
    projected_limbs.iter().all(|limb| {
        match limb
            .pixel_polygon
            .as_slice()
            .windows(2)
            .find(|points| points[0].x <= pixel_position.x && points[1].x >= pixel_position.x)
        {
            Some(points) => {
                if points[0].x == points[1].x {
                    return (pixel_position.y) < f32::min(points[0].y, points[1].y);
                }

                // since Y is pointing downwards, "is above" is actually !Line::is_above()
                !Line(points[0], points[1]).is_above(point![pixel_position.x, pixel_position.y])
            }
            None => true,
        }
    })
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct ProjectedLimbs {
    pub limbs: Vec<Limb>,
}
