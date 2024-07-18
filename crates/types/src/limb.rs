use std::fmt::Debug;

use geometry::line::Line;
use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Limb {
    pub pixel_polygon: Vec<Point2<Pixel>>,
}

pub fn project_onto_limbs(position: Point2<Pixel>, projected_limbs: &[Limb]) -> Option<f32> {
    projected_limbs
        .iter()
        .flat_map(|limb| {
            limb.pixel_polygon
                .as_slice()
                .windows(2)
                .filter_map(|points| {
                    let is_outside_of_segment =
                        position.x() < points[0].x() || position.x() > points[1].x();
                    if is_outside_of_segment {
                        return None;
                    }

                    let is_vertical_segment = points[0].x() == points[1].x();
                    if is_vertical_segment {
                        return Some(f32::min(points[0].y(), points[1].y()));
                    }

                    Some(
                        Line::from_points(points[0], points[1]).project_onto_along_y_axis(position),
                    )
                })
                .min_by(f32::total_cmp)
        })
        .min_by(f32::total_cmp)
}

pub fn is_above_limbs(position: Point2<Pixel>, projected_limbs: &[Limb]) -> bool {
    project_onto_limbs(position, projected_limbs).map_or(true, |projected_position_y| {
        position.y() < projected_position_y
    })
}

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ProjectedLimbs {
    pub limbs: Vec<Limb>,
}

#[cfg(test)]
mod tests {
    use linear_algebra::point;

    use super::*;

    #[test]
    fn left_limb_is_ignored() {
        let position = point![2.0, 0.0];
        let projected_limbs = vec![Limb {
            pixel_polygon: vec![point![0.0, 0.0], point![1.0, 1.0]],
        }];
        assert!(is_above_limbs(position, &projected_limbs));
    }

    #[test]
    fn right_limb_is_ignored() {
        let position = point![2.0, 0.0];
        let projected_limbs = vec![Limb {
            pixel_polygon: vec![point![3.0, 0.0], point![4.0, 1.0]],
        }];
        assert!(is_above_limbs(position, &projected_limbs));
    }

    #[test]
    fn too_low_limb_leads_to_point_being_above() {
        let position = point![2.0, 0.0];
        let projected_limbs = vec![Limb {
            pixel_polygon: vec![point![1.0, 10.0], point![3.0, 11.0]],
        }];
        assert!(is_above_limbs(position, &projected_limbs));
    }

    #[test]
    fn high_limb_leads_to_point_being_below() {
        let position = point![2.0, 10.0];
        let projected_limbs = vec![Limb {
            pixel_polygon: vec![point![1.0, 0.0], point![3.0, 1.0]],
        }];
        assert!(!is_above_limbs(position, &projected_limbs));
    }
}
