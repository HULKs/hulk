use std::vec::IntoIter;

use geometry::{
    line::{Line, Line2},
    line_segment::LineSegment,
    two_lines::TwoLines,
    Distance,
};
use linear_algebra::{Point2, Rotation2};
use ordered_float::NotNan;
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub enum RansacFeature<Frame> {
    #[default]
    None,
    Line(Line2<Frame>),
    TwoLines(TwoLines<Frame>),
}

impl<Frame> RansacFeature<Frame> {
    fn from_points(point1: Point2<Frame>, point2: Point2<Frame>, point3: Point2<Frame>) -> Self {
        let line = Line::from_points(point1, point2);
        let two_lines = TwoLines::from_line_and_point_orthogonal(&line, point3);

        if two_lines.second_direction.norm() > f32::EPSILON {
            Self::TwoLines(two_lines)
        } else {
            Self::Line(line)
        }
    }

    fn score<'a>(
        &self,
        unused_points: impl IntoIterator<Item = &'a Point2<Frame>>,
        maximum_score_distance: f32,
        maximum_score_distance_squared: f32,
    ) -> f32
    where
        Frame: 'a,
    {
        unused_points
            .into_iter()
            .filter(|&point| self.squared_distance_to(*point) <= maximum_score_distance_squared)
            .map(|point| 1.0 - self.distance_to(*point) / maximum_score_distance)
            .sum()
    }
}

impl<Frame> Distance<Point2<Frame>> for RansacFeature<Frame> {
    fn squared_distance_to(&self, point: Point2<Frame>) -> f32 {
        match self {
            RansacFeature::None => f32::INFINITY,
            RansacFeature::Line(line) => line.squared_distance_to(point),
            RansacFeature::TwoLines(two_lines) => two_lines.squared_distance_to(point),
        }
    }
}

impl<Frame> IntoIterator for RansacResult<Frame> {
    type Item = RansacLineSegment<Frame>;

    type IntoIter = IntoIter<RansacLineSegment<Frame>>;

    fn into_iter(self) -> Self::IntoIter {
        match self.feature {
            RansacFeature::None => Vec::new(),
            RansacFeature::Line(line) => {
                RansacLineSegment::try_from_used_points(line, self.used_points)
                    .map(|line_segment| vec![line_segment])
                    .unwrap_or_default()
            }
            RansacFeature::TwoLines(two_lines) => {
                let dividing_line1 = Line {
                    point: two_lines.intersection_point,
                    direction: two_lines.first_direction.normalize()
                        + two_lines.second_direction.normalize(),
                };
                let dividing_line2 = Line {
                    point: two_lines.intersection_point,
                    direction: two_lines.first_direction.normalize()
                        - two_lines.second_direction.normalize(),
                };

                let (used_points1, used_points2): (Vec<_>, Vec<_>) =
                    self.used_points.iter().partition(|&point| {
                        dividing_line1.is_above(*point) != dividing_line2.is_above(*point)
                    });
                let line1 = Line {
                    point: two_lines.intersection_point,
                    direction: two_lines.first_direction,
                };
                let line2 = Line {
                    point: two_lines.intersection_point,
                    direction: two_lines.second_direction,
                };

                [
                    RansacLineSegment::try_from_used_points(line1, used_points1),
                    RansacLineSegment::try_from_used_points(line2, used_points2),
                ]
                .into_iter()
                .flatten()
                .collect()
            }
        }
        .into_iter()
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct RansacLineSegment<Frame> {
    pub line_segment: LineSegment<Frame>,
    pub sorted_used_points: Vec<Point2<Frame>>,
}

impl<Frame> RansacLineSegment<Frame> {
    fn try_from_used_points(
        line: Line2<Frame>,
        mut used_points: Vec<Point2<Frame>>,
    ) -> Option<Self> {
        struct Horizontal;

        let frame_to_horizontal: Rotation2<Frame, Horizontal> =
            Rotation2::from_vector(line.direction).inverse();

        used_points.sort_by_key(|point| {
            let x = (frame_to_horizontal * point).x();
            NotNan::new(x).expect("X coordinate should never be NaN")
        });

        let point1 = line.closest_point(used_points.first().copied()?);
        let point2 = line.closest_point(used_points.last().copied()?);

        Some(Self {
            line_segment: LineSegment(point1, point2),
            sorted_used_points: used_points,
        })
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct RansacResult<Frame> {
    pub feature: RansacFeature<Frame>,
    pub used_points: Vec<Point2<Frame>>,
}

pub struct Ransac<Frame> {
    pub unused_points: Vec<Point2<Frame>>,
}

impl<Frame> Ransac<Frame> {
    pub fn new(unused_points: Vec<Point2<Frame>>) -> Ransac<Frame> {
        Ransac { unused_points }
    }
}

impl<Frame> Ransac<Frame> {
    pub fn next_feature(
        &mut self,
        random_number_generator: &mut impl Rng,
        iterations: usize,
        fit_two_lines: bool,
        maximum_score_distance: f32,
        maximum_inclusion_distance: f32,
    ) -> RansacResult<Frame> {
        if self.unused_points.len() < 2 {
            return RansacResult {
                feature: RansacFeature::None,
                used_points: vec![],
            };
        }
        if self.unused_points.len() == 2 {
            return RansacResult {
                feature: RansacFeature::Line(Line::from_points(
                    self.unused_points[0],
                    self.unused_points[1],
                )),
                used_points: self.unused_points.clone(),
            };
        }

        let maximum_score_distance_squared = maximum_score_distance * maximum_score_distance;
        let maximum_inclusion_distance_squared =
            maximum_inclusion_distance * maximum_inclusion_distance;

        let best_feature = (0..iterations)
            .map(|_| {
                let feature = if fit_two_lines {
                    let mut points = self
                        .unused_points
                        .choose_multiple(random_number_generator, 3);

                    RansacFeature::from_points(
                        *points.next().unwrap(),
                        *points.next().unwrap(),
                        *points.next().unwrap(),
                    )
                } else {
                    let mut points = self
                        .unused_points
                        .choose_multiple(random_number_generator, 2);

                    RansacFeature::Line(Line::from_points(
                        *points.next().unwrap(),
                        *points.next().unwrap(),
                    ))
                };

                let score = feature.score(
                    self.unused_points.iter(),
                    maximum_score_distance,
                    maximum_score_distance_squared,
                );
                (feature, score)
            })
            .max_by_key(|(_feature, score)| NotNan::new(*score).expect("score should never be NaN"))
            .expect("max_by_key erroneously returned no result")
            .0;

        let (used_points, unused_points) = self.unused_points.iter().partition(|point| {
            best_feature.squared_distance_to(**point) <= maximum_inclusion_distance_squared
        });
        self.unused_points = unused_points;

        RansacResult {
            feature: best_feature,
            used_points,
        }
    }
}

#[cfg(test)]
mod test {
    use approx::{assert_relative_eq, relative_eq};
    use linear_algebra::point;
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;

    use super::*;

    #[derive(Debug, PartialEq, Eq, Default)]
    struct SomeFrame;

    #[test]
    fn ransac_empty_input() {
        let mut ransac = Ransac::<SomeFrame>::new(vec![]);
        let mut rng = ChaChaRng::from_entropy();
        assert_eq!(
            ransac.next_feature(&mut rng, 10, false, 5.0, 5.0),
            RansacResult::default()
        );
    }

    #[test]
    fn ransac_single_point() {
        let mut ransac = Ransac::<SomeFrame>::new(vec![]);
        let mut rng = ChaChaRng::from_entropy();
        assert_eq!(
            ransac.next_feature(&mut rng, 10, false, 5.0, 5.0),
            RansacResult::default()
        );
    }

    #[test]
    fn ransac_two_points() {
        let p1 = point![15.0, 15.0];
        let p2 = point![30.0, 30.0];
        let mut ransac = Ransac::<SomeFrame>::new(vec![p1, p2]);
        let mut rng = ChaChaRng::from_entropy();
        let RansacResult {
            feature,
            used_points,
        } = ransac.next_feature(&mut rng, 10, false, 5.0, 5.0);
        println!("{feature:#?}");
        println!("{used_points:#?}");

        if let RansacFeature::Line(line) = feature {
            assert!(
                relative_eq!(line, Line::from_points(p1, p2))
                    || relative_eq!(line, Line::from_points(p2, p1))
            );
            assert!(relative_eq!(used_points[0], p1) || relative_eq!(used_points[0], p2));
            assert!(relative_eq!(used_points[1], p2) || relative_eq!(used_points[0], p1));
        } else {
            panic!("expected line")
        }
    }

    #[test]
    fn ransac_perfect_line() {
        let slope = 5.3;
        let y_intercept = -83.1;
        let points: Vec<_> = (0..100)
            .map(|x| point![x as f32, y_intercept + x as f32 * slope])
            .collect();

        let mut ransac = Ransac::<SomeFrame>::new(points.clone());
        let mut rng = ChaChaRng::from_entropy();
        let result = ransac.next_feature(&mut rng, 15, false, 1.0, 1.0);

        if let RansacFeature::Line(line) = result.feature {
            assert_relative_eq!(line.slope(), slope, epsilon = 0.0001);
            assert_relative_eq!(line.y_axis_intercept(), y_intercept, epsilon = 0.0001);
            assert_eq!(result.used_points, points);
        } else {
            panic!("expected line")
        }
    }
}
