use std::collections::{BTreeMap, HashMap};

use itertools::Itertools;
use linear_algebra::Point2;
use types::field_dimensions::{FieldDimensions, Half, Side};

use coordinate_systems::Field;

use super::{FEATURE_CLASSES, VisualFeatureClass};

const SYMMETRY_EPSILON: f32 = 1.0e-4;
pub(crate) const TRIPLET_BIN_SIZE: f32 = 0.12;
const MIN_MAP_TRIPLET_ABS_BETA: f32 = 0.03;

#[derive(Clone, Debug)]
pub(crate) struct LandmarkMap {
    pub landmarks: Vec<Landmark>,
    landmarks_by_class: BTreeMap<VisualFeatureClass, Vec<usize>>,
    pub map_triplets_by_bin: HashMap<MapTripletBin, Vec<MapTriplet>>,
    class_rarity_weight: BTreeMap<VisualFeatureClass, f32>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Landmark {
    pub id: usize,
    pub symmetric_id: usize,
    pub class: VisualFeatureClass,
    pub xy: Point2<Field>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MapTriplet {
    pub xy_a: Point2<Field>,
    pub pair_dist: f32,
    pub pair_angle: f32,
    pub alpha: f32,
    pub beta: f32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct MapTripletBin {
    pub classes: [VisualFeatureClass; 3],
    pub alpha_bin: i16,
    pub beta_bin: i16,
}

impl LandmarkMap {
    pub fn new(field: &FieldDimensions, min_map_baseline: f32) -> Self {
        let mut landmarks = candidate_landmarks(field);
        fill_symmetric_ids(&mut landmarks);

        let mut landmarks_by_class = FEATURE_CLASSES
            .into_iter()
            .map(|class| (class, Vec::new()))
            .collect::<BTreeMap<_, _>>();
        for landmark in &landmarks {
            landmarks_by_class
                .entry(landmark.class)
                .or_default()
                .push(landmark.id);
        }

        let mut map_triplets_by_bin = HashMap::new();
        for a in &landmarks {
            for b in &landmarks {
                if a.id == b.id {
                    continue;
                }
                let v = (b.xy - a.xy).inner;
                let pair_dist = v.norm();
                if pair_dist < min_map_baseline {
                    continue;
                }
                let base_norm_squared = v.norm_squared();
                for c in &landmarks {
                    if c.id == a.id || c.id == b.id {
                        continue;
                    }
                    let w = (c.xy - a.xy).inner;
                    let alpha = w.dot(&v) / base_norm_squared;
                    let beta = cross(v, w) / base_norm_squared;
                    if beta.abs() < MIN_MAP_TRIPLET_ABS_BETA {
                        continue;
                    }
                    map_triplets_by_bin
                        .entry(MapTripletBin::new(a.class, b.class, c.class, alpha, beta))
                        .or_insert_with(Vec::new)
                        .push(MapTriplet {
                            xy_a: a.xy,
                            pair_dist,
                            pair_angle: v.y.atan2(v.x),
                            alpha,
                            beta,
                        });
                }
            }
        }

        let class_rarity_weight = FEATURE_CLASSES
            .into_iter()
            .map(|class| {
                let count = landmarks_by_class.get(&class).map_or(0, Vec::len);
                let weight = if count == 0 { 0.0 } else { 1.0 / count as f32 };
                (class, weight)
            })
            .collect();

        Self {
            landmarks,
            landmarks_by_class,
            map_triplets_by_bin,
            class_rarity_weight,
        }
    }

    pub fn symmetric_id(&self, landmark_id: usize) -> usize {
        self.landmarks
            .get(landmark_id)
            .map_or(landmark_id, |landmark| landmark.symmetric_id)
    }

    pub fn has_class(&self, class: VisualFeatureClass) -> bool {
        !self.landmarks_for_class(class).is_empty()
    }

    pub fn landmarks_for_class(&self, class: VisualFeatureClass) -> &[usize] {
        self.landmarks_by_class
            .get(&class)
            .map_or(&[], Vec::as_slice)
    }

    pub fn rarity_weight(&self, class: VisualFeatureClass) -> f32 {
        self.class_rarity_weight.get(&class).copied().unwrap_or(0.0)
    }
}

impl MapTripletBin {
    pub fn new(
        class_a: VisualFeatureClass,
        class_b: VisualFeatureClass,
        class_c: VisualFeatureClass,
        alpha: f32,
        beta: f32,
    ) -> Self {
        Self {
            classes: [class_a, class_b, class_c],
            alpha_bin: triplet_bin(alpha),
            beta_bin: triplet_bin(beta),
        }
    }
}

pub(crate) fn triplet_bin(value: f32) -> i16 {
    (value / TRIPLET_BIN_SIZE)
        .round()
        .clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

fn cross(left: nalgebra::Vector2<f32>, right: nalgebra::Vector2<f32>) -> f32 {
    left.x * right.y - left.y * right.x
}

fn candidate_landmarks(field: &FieldDimensions) -> Vec<Landmark> {
    candidate_points(field)
        .into_iter()
        .enumerate()
        .map(|(id, (class, xy))| Landmark {
            id,
            symmetric_id: id,
            class,
            xy,
        })
        .collect()
}

fn candidate_points(field: &FieldDimensions) -> Vec<(VisualFeatureClass, Point2<Field>)> {
    let mut points = Vec::new();
    points.extend(
        goalpost_candidates(field)
            .into_iter()
            .map(|point| (VisualFeatureClass::GoalPost, point)),
    );
    points.extend(
        l_spot_candidates(field)
            .into_iter()
            .map(|point| (VisualFeatureClass::LSpot, point)),
    );
    points.extend(
        t_spot_candidates(field)
            .into_iter()
            .map(|point| (VisualFeatureClass::TSpot, point)),
    );
    points.extend(
        x_spot_candidates(field)
            .into_iter()
            .map(|point| (VisualFeatureClass::XSpot, point)),
    );
    points.extend(
        penalty_spot_candidates(field)
            .into_iter()
            .map(|point| (VisualFeatureClass::PenaltySpot, point)),
    );
    points
}

fn fill_symmetric_ids(landmarks: &mut [Landmark]) {
    for index in 0..landmarks.len() {
        let landmark = landmarks[index];
        if let Some(partner) = landmarks.iter().find(|candidate| {
            candidate.class == landmark.class
                && (candidate.xy.x() + landmark.xy.x()).abs() <= SYMMETRY_EPSILON
                && (candidate.xy.y() + landmark.xy.y()).abs() <= SYMMETRY_EPSILON
        }) {
            landmarks[index].symmetric_id = partner.id;
        }
    }
}

fn goalpost_candidates(field: &FieldDimensions) -> Vec<Point2<Field>> {
    [Half::Opponent, Half::Own]
        .into_iter()
        .cartesian_product([Side::Left, Side::Right])
        .map(|(half, side)| field.goal_post(half, side))
        .collect()
}

fn l_spot_candidates(field: &FieldDimensions) -> Vec<Point2<Field>> {
    [Half::Opponent, Half::Own]
        .into_iter()
        .cartesian_product([Side::Left, Side::Right])
        .flat_map(|(half, side)| {
            [
                field.corner(half, side),
                field.goal_box_corner(half, side),
                field.penalty_box_corner(half, side),
            ]
        })
        .collect()
}

fn t_spot_candidates(field: &FieldDimensions) -> Vec<Point2<Field>> {
    [Side::Left, Side::Right]
        .into_iter()
        .map(|side| field.t_crossing(side))
        .chain(
            [Half::Opponent, Half::Own]
                .into_iter()
                .cartesian_product([Side::Left, Side::Right])
                .flat_map(|(half, side)| {
                    [
                        field.goal_box_goal_line_intersection(half, side),
                        field.penalty_box_goal_line_intersection(half, side),
                    ]
                }),
        )
        .collect()
}

fn x_spot_candidates(field: &FieldDimensions) -> Vec<Point2<Field>> {
    [
        field.center(),
        field.x_crossing(Side::Left),
        field.x_crossing(Side::Right),
    ]
    .into()
}

fn penalty_spot_candidates(field: &FieldDimensions) -> Vec<Point2<Field>> {
    [Half::Opponent, Half::Own]
        .into_iter()
        .map(|half| field.penalty_spot(half))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_sets_match_field_feature_classes() {
        let map = LandmarkMap::new(&FieldDimensions::SPL_2025, 0.25);

        assert_eq!(
            map.landmarks_for_class(VisualFeatureClass::GoalPost).len(),
            4
        );
        assert_eq!(map.landmarks_for_class(VisualFeatureClass::LSpot).len(), 12);
        assert_eq!(map.landmarks_for_class(VisualFeatureClass::TSpot).len(), 10);
        assert_eq!(map.landmarks_for_class(VisualFeatureClass::XSpot).len(), 3);
        assert_eq!(
            map.landmarks_for_class(VisualFeatureClass::PenaltySpot)
                .len(),
            2
        );
    }

    #[test]
    fn triplet_lookup_contains_non_collinear_landmark_shapes() {
        let map = LandmarkMap::new(&FieldDimensions::SPL_2025, 0.25);

        assert!(!map.map_triplets_by_bin.is_empty());
    }
}
