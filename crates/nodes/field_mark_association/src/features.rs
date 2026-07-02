use coordinate_systems::Pixel;
use linear_algebra::{Point2, point};
use types::object_detection::{Object, RobocupObjectLabel};

/// Extracts goalpost image points from object detections.
pub fn find_detected_goalposts(detections: &[Object<RobocupObjectLabel>]) -> Vec<Point2<Pixel>> {
    find_detected_visual_features(detections)
        .goalposts
        .into_iter()
        .map(|feature| feature.pixel)
        .collect()
}

/// Field-feature detection used by global localization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectedVisualFeature {
    /// Image point used for projection and association.
    pub pixel: Point2<Pixel>,
    /// Detector confidence in `[0, 1]`; invalid or low-confidence detections are ignored later.
    pub confidence: f32,
}

impl DetectedVisualFeature {
    fn new(pixel: Point2<Pixel>, confidence: f32) -> Self {
        Self { pixel, confidence }
    }
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

/// Extracts all field-feature detections supported by global localization.
pub fn find_detected_visual_features(
    detections: &[Object<RobocupObjectLabel>],
) -> DetectedVisualFeatures {
    detections
        .iter()
        .fold(DetectedVisualFeatures::default(), |mut features, object| {
            let confidence = object.bounding_box.confidence;
            match object.label {
                RobocupObjectLabel::GoalPost => features.goalposts.push(
                    DetectedVisualFeature::new(pixel_bottom_center(object), confidence),
                ),
                RobocupObjectLabel::LSpot => features
                    .l_spots
                    .push(DetectedVisualFeature::new(pixel_center(object), confidence)),
                RobocupObjectLabel::TSpot => features
                    .t_spots
                    .push(DetectedVisualFeature::new(pixel_center(object), confidence)),
                RobocupObjectLabel::PenaltySpot => features
                    .penalty_spots
                    .push(DetectedVisualFeature::new(pixel_center(object), confidence)),
                RobocupObjectLabel::XSpot => features
                    .x_spots
                    .push(DetectedVisualFeature::new(pixel_center(object), confidence)),
                _ => {}
            }
            features
        })
}

fn pixel_bottom_center(object: &Object<RobocupObjectLabel>) -> Point2<Pixel> {
    let area = object.bounding_box.area;
    point![(area.min.x() + area.max.x()) * 0.5, area.max.y()]
}

fn pixel_center(object: &Object<RobocupObjectLabel>) -> Point2<Pixel> {
    let area = object.bounding_box.area;
    point![
        (area.min.x() + area.max.x()) * 0.5,
        (area.min.y() + area.max.y()) * 0.5
    ]
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

        let goalposts = find_detected_goalposts(&detections);

        assert_eq!(goalposts.len(), 1);
        assert_eq!(goalposts[0], point![20.0, 50.0]);
    }

    #[test]
    fn spot_detections_use_pixel_center() {
        let detections = vec![
            Object {
                label: RobocupObjectLabel::LSpot,
                bounding_box: BoundingBox {
                    area: Rectangle {
                        min: point![10.0, 20.0],
                        max: point![30.0, 50.0],
                    },
                    confidence: 1.0,
                },
            },
            Object {
                label: RobocupObjectLabel::TSpot,
                bounding_box: BoundingBox {
                    area: Rectangle {
                        min: point![40.0, 60.0],
                        max: point![60.0, 80.0],
                    },
                    confidence: 1.0,
                },
            },
            Object {
                label: RobocupObjectLabel::PenaltySpot,
                bounding_box: BoundingBox {
                    area: Rectangle {
                        min: point![70.0, 90.0],
                        max: point![90.0, 110.0],
                    },
                    confidence: 1.0,
                },
            },
        ];

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
