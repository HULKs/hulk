use linear_algebra::Point2;
use types::field_dimensions::{FieldDimensions, Half, Side};

use coordinate_systems::Field;

use crate::features::{FEATURE_CLASSES, VisualFeatureClass};

#[derive(Clone, Debug)]
pub(crate) struct LandmarkMap {
    pub landmarks: Vec<Landmark>,
    landmarks_by_class: [Vec<usize>; FEATURE_CLASSES.len()],
    class_rarity_weight: [f32; FEATURE_CLASSES.len()],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Landmark {
    pub symmetric_id: usize,
    pub class: VisualFeatureClass,
    pub xy: Point2<Field>,
}

impl LandmarkMap {
    pub fn new(field: &FieldDimensions, symmetry_epsilon: f32) -> Self {
        let mut landmarks = candidate_landmarks(field);
        fill_symmetric_ids(&mut landmarks, symmetry_epsilon);
        let landmarks_by_class = landmarks_by_class(&landmarks);
        let class_rarity_weight = class_rarity_weights(&landmarks_by_class);

        Self {
            landmarks,
            landmarks_by_class,
            class_rarity_weight,
        }
    }

    pub fn symmetric_id(&self, landmark_id: usize) -> usize {
        self.landmarks[landmark_id].symmetric_id
    }

    pub fn landmarks_for_class(&self, class: VisualFeatureClass) -> &[usize] {
        &self.landmarks_by_class[class.index()]
    }

    pub fn rarity_weight(&self, class: VisualFeatureClass) -> f32 {
        self.class_rarity_weight[class.index()]
    }
}

fn landmarks_by_class(landmarks: &[Landmark]) -> [Vec<usize>; FEATURE_CLASSES.len()] {
    let mut landmarks_by_class = std::array::from_fn(|_| Vec::new());
    for (id, landmark) in landmarks.iter().enumerate() {
        landmarks_by_class[landmark.class.index()].push(id);
    }
    landmarks_by_class
}

fn class_rarity_weights(
    landmarks_by_class: &[Vec<usize>; FEATURE_CLASSES.len()],
) -> [f32; FEATURE_CLASSES.len()] {
    std::array::from_fn(|index| {
        let count = landmarks_by_class[index].len();
        if count == 0 { 0.0 } else { 1.0 / count as f32 }
    })
}

fn candidate_landmarks(field: &FieldDimensions) -> Vec<Landmark> {
    candidate_points(field)
        .into_iter()
        .enumerate()
        .map(|(id, (class, xy))| Landmark {
            symmetric_id: id,
            class,
            xy,
        })
        .collect()
}

pub fn candidate_points(field: &FieldDimensions) -> Vec<(VisualFeatureClass, Point2<Field>)> {
    let mut points = Vec::new();
    for half in [Half::Opponent, Half::Own] {
        for side in [Side::Left, Side::Right] {
            points.push((VisualFeatureClass::GoalPost, field.goal_post(half, side)));
        }
    }
    for half in [Half::Opponent, Half::Own] {
        for side in [Side::Left, Side::Right] {
            points.push((VisualFeatureClass::LSpot, field.corner(half, side)));
            points.push((VisualFeatureClass::LSpot, field.goal_box_corner(half, side)));
            points.push((
                VisualFeatureClass::LSpot,
                field.penalty_box_corner(half, side),
            ));
        }
    }
    for side in [Side::Left, Side::Right] {
        points.push((VisualFeatureClass::TSpot, field.t_crossing(side)));
    }
    for half in [Half::Opponent, Half::Own] {
        for side in [Side::Left, Side::Right] {
            points.push((
                VisualFeatureClass::TSpot,
                field.goal_box_goal_line_intersection(half, side),
            ));
            points.push((
                VisualFeatureClass::TSpot,
                field.penalty_box_goal_line_intersection(half, side),
            ));
        }
    }
    points.push((VisualFeatureClass::XSpot, field.center()));
    for side in [Side::Left, Side::Right] {
        points.push((VisualFeatureClass::XSpot, field.x_crossing(side)));
    }
    for half in [Half::Opponent, Half::Own] {
        points.push((VisualFeatureClass::PenaltySpot, field.penalty_spot(half)));
    }
    points
}

fn fill_symmetric_ids(landmarks: &mut [Landmark], symmetry_epsilon: f32) {
    for index in 0..landmarks.len() {
        let landmark = landmarks[index];
        if let Some(partner_id) = landmarks.iter().position(|candidate| {
            candidate.class == landmark.class
                && (candidate.xy.x() + landmark.xy.x()).abs() <= symmetry_epsilon
                && (candidate.xy.y() + landmark.xy.y()).abs() <= symmetry_epsilon
        }) {
            landmarks[index].symmetric_id = partner_id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetry_tolerance_controls_partner_matching() {
        let mut landmarks = [
            Landmark {
                symmetric_id: 0,
                class: VisualFeatureClass::LSpot,
                xy: linear_algebra::point![1.0, 2.0],
            },
            Landmark {
                symmetric_id: 1,
                class: VisualFeatureClass::LSpot,
                xy: linear_algebra::point![-1.001, -2.0],
            },
        ];
        fill_symmetric_ids(
            &mut landmarks,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        assert_eq!(landmarks[0].symmetric_id, 0);
        fill_symmetric_ids(&mut landmarks, 0.002);
        assert_eq!(landmarks[0].symmetric_id, 1);
        assert_eq!(landmarks[1].symmetric_id, 0);
    }

    #[test]
    fn candidate_sets_match_field_feature_classes() {
        let map = LandmarkMap::new(
            &FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        assert_eq!(
            map.landmarks.len(),
            candidate_points(&FieldDimensions::SPL_2025).len()
        );
        for (id, landmark) in map.landmarks.iter().enumerate() {
            let partner = &map.landmarks[landmark.symmetric_id];
            assert_eq!(partner.symmetric_id, id);
            assert_eq!(partner.class, landmark.class);
            assert!((partner.xy.coords().inner + landmark.xy.coords().inner).norm() < 1.0e-4);
        }

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
}
