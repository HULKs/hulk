use coordinate_systems::Pixel;
use linear_algebra::{Point2, point};
use types::object_detection::{Object, RobocupObjectLabel};

pub(crate) const FEATURE_CLASSES: [VisualFeatureClass; 5] = [
    VisualFeatureClass::GoalPost,
    VisualFeatureClass::LSpot,
    VisualFeatureClass::TSpot,
    VisualFeatureClass::XSpot,
    VisualFeatureClass::PenaltySpot,
];

/// Field-feature classes supported by association.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum VisualFeatureClass {
    /// Upright goalpost landmark detected at its field-contact point.
    GoalPost,
    /// L-shaped line crossing landmark.
    LSpot,
    /// T-shaped line crossing landmark.
    TSpot,
    /// X-shaped line crossing landmark.
    XSpot,
    /// Penalty marker landmark.
    PenaltySpot,
}

impl VisualFeatureClass {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::GoalPost => 0,
            Self::LSpot => 1,
            Self::TSpot => 2,
            Self::XSpot => 3,
            Self::PenaltySpot => 4,
        }
    }
}

/// Field-feature detection used by global localization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectedVisualFeature {
    /// Image point used for projection and association.
    pub pixel: Point2<Pixel>,
    /// Detector confidence in `[0, 1]`; invalid or low-confidence detections are ignored later.
    pub confidence: f32,
}

/// Field-feature detections grouped by the landmark class used by global localization.
#[derive(Debug, Default, PartialEq)]
pub struct DetectedVisualFeatures {
    /// Goalpost detections, represented by bottom-center image points.
    pub goalposts: Vec<DetectedVisualFeature>,
    /// L-crossing spot detections, represented by bounding-box centers.
    pub l_spots: Vec<DetectedVisualFeature>,
    /// T-crossing spot detections, represented by bounding-box centers.
    pub t_spots: Vec<DetectedVisualFeature>,
    /// Penalty spot detections, represented by bounding-box centers.
    pub penalty_spots: Vec<DetectedVisualFeature>,
    /// x-spot detections, represented by bounding-box centers.
    pub x_spots: Vec<DetectedVisualFeature>,
}

impl DetectedVisualFeatures {
    /// Counts detections from classes supported by global localization.
    pub fn supported_feature_count(&self) -> usize {
        self.goalposts.len()
            + self.l_spots.len()
            + self.t_spots.len()
            + self.penalty_spots.len()
            + self.x_spots.len()
    }
}

/// Iterates class-grouped detections in the same order used by association.
pub fn raw_detections(
    features: &DetectedVisualFeatures,
) -> impl Iterator<Item = (VisualFeatureClass, DetectedVisualFeature)> + '_ {
    [
        (VisualFeatureClass::GoalPost, features.goalposts.as_slice()),
        (VisualFeatureClass::LSpot, features.l_spots.as_slice()),
        (VisualFeatureClass::TSpot, features.t_spots.as_slice()),
        (VisualFeatureClass::XSpot, features.x_spots.as_slice()),
        (
            VisualFeatureClass::PenaltySpot,
            features.penalty_spots.as_slice(),
        ),
    ]
    .into_iter()
    .flat_map(|(class, features)| features.iter().map(move |f| (class, *f)))
}

/// Extracts all field-feature detections supported by global localization.
pub fn find_detected_visual_features(
    detections: &[Object<RobocupObjectLabel>],
) -> DetectedVisualFeatures {
    let mut features = DetectedVisualFeatures::default();
    for object in detections {
        let confidence = object.bounding_box.confidence;
        let destination = match object.label {
            RobocupObjectLabel::GoalPost => &mut features.goalposts,
            RobocupObjectLabel::LSpot => &mut features.l_spots,
            RobocupObjectLabel::TSpot => &mut features.t_spots,
            RobocupObjectLabel::PenaltySpot => &mut features.penalty_spots,
            RobocupObjectLabel::XSpot => &mut features.x_spots,
            _ => continue,
        };
        let pixel = if object.label == RobocupObjectLabel::GoalPost {
            pixel_bottom_center(object)
        } else {
            object.bounding_box.area.center()
        };
        destination.push(DetectedVisualFeature { pixel, confidence });
    }
    features
}

fn pixel_bottom_center(object: &Object<RobocupObjectLabel>) -> Point2<Pixel> {
    let area = object.bounding_box.area;
    point![(area.min.x() + area.max.x()) * 0.5, area.max.y()]
}

#[cfg(test)]
mod tests {
    use geometry::rectangle::Rectangle;
    use types::bounding_box::BoundingBox;

    use super::*;

    #[test]
    fn goalpost_detection_uses_pixel_bottom_center() {
        let detections = vec![Object {
            label: RobocupObjectLabel::GoalPost,
            bounding_box: BoundingBox {
                area: Rectangle {
                    min: point![10.0, 20.0],
                    max: point![30.0, 50.0],
                },
                confidence: 1.0,
            },
        }];

        let goalposts = find_detected_visual_features(&detections).goalposts;

        assert_eq!(goalposts.len(), 1);
        assert_eq!(goalposts[0].pixel, point![20.0, 50.0]);
    }

    #[test]
    fn spot_detections_use_pixel_center() {
        let detections = [
            (
                RobocupObjectLabel::LSpot,
                point![10.0, 20.0],
                point![30.0, 50.0],
            ),
            (
                RobocupObjectLabel::TSpot,
                point![40.0, 60.0],
                point![60.0, 80.0],
            ),
            (
                RobocupObjectLabel::PenaltySpot,
                point![70.0, 90.0],
                point![90.0, 110.0],
            ),
        ]
        .map(|(label, min, max)| Object {
            label,
            bounding_box: BoundingBox {
                area: Rectangle { min, max },
                confidence: 1.0,
            },
        });

        let features = find_detected_visual_features(&detections);

        assert_eq!(feature_pixels(&features.l_spots), vec![point![20.0, 35.0]]);
        assert_eq!(feature_pixels(&features.t_spots), vec![point![50.0, 70.0]]);
        assert_eq!(
            feature_pixels(&features.penalty_spots),
            vec![point![80.0, 100.0]]
        );
        assert_eq!(
            features.l_spots.first().map(|feature| feature.confidence),
            Some(1.0)
        );
    }

    fn feature_pixels(features: &[DetectedVisualFeature]) -> Vec<Point2<Pixel>> {
        features.iter().map(|feature| feature.pixel).collect()
    }
}
