use geometry::{
    corner::Corner,
    line::{Line, Line2},
    Distance,
};
use linear_algebra::Point2;
use ordered_float::NotNan;
use rand::{seq::SliceRandom, Rng};

#[derive(Default, Debug, PartialEq)]
pub enum RansacFeature<Frame> {
    #[default]
    None,
    Line(Line2<Frame>),
    Corner(Corner<Frame>),
}

impl<Frame> RansacFeature<Frame> {
    fn from_points(point1: Point2<Frame>, point2: Point2<Frame>, point3: Point2<Frame>) -> Self {
        let line = Line::from_points(point1, point2);
        let corner = Corner::from_line_and_point_orthogonal(&line, point3);

        if corner.direction2.norm() > f32::EPSILON {
            Self::Corner(corner)
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
            .filter(|point| self.squared_distance_to(**point) <= maximum_score_distance_squared)
            .map(|point| 1.0 - self.distance_to(*point) / maximum_score_distance)
            .sum()
    }
}

impl<Frame> Distance<Point2<Frame>> for RansacFeature<Frame> {
    fn squared_distance_to(&self, point: Point2<Frame>) -> f32 {
        match self {
            RansacFeature::None => f32::INFINITY,
            RansacFeature::Line(line) => line.squared_distance_to(point),
            RansacFeature::Corner(corner) => corner.squared_distance_to(point),
        }
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
                feature: RansacFeature::Line(Line(self.unused_points[0], self.unused_points[1])),
                used_points: self.unused_points.clone(),
            };
        }

        let maximum_score_distance_squared = maximum_score_distance * maximum_score_distance;
        let maximum_inclusion_distance_squared =
            maximum_inclusion_distance * maximum_inclusion_distance;

        let best_feature = (0..iterations)
            .map(|_| {
                let mut points = self
                    .unused_points
                    .choose_multiple(random_number_generator, 3);
                let feature = RansacFeature::from_points(
                    *points.next().unwrap(),
                    *points.next().unwrap(),
                    *points.next().unwrap(),
                );
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
            ransac.next_feature(&mut rng, 10, 5.0, 5.0),
            RansacResult::default()
        );
    }

    #[test]
    fn ransac_single_point() {
        let mut ransac = Ransac::<SomeFrame>::new(vec![]);
        let mut rng = ChaChaRng::from_entropy();
        assert_eq!(
            ransac.next_feature(&mut rng, 10, 5.0, 5.0),
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
        } = ransac.next_feature(&mut rng, 10, 5.0, 5.0);
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
        let result = ransac.next_feature(&mut rng, 15, 1.0, 1.0);

        if let RansacFeature::Line(line) = result.feature {
            assert_relative_eq!(line.slope(), slope, epsilon = 0.0001);
            assert_relative_eq!(line.y_axis_intercept(), y_intercept, epsilon = 0.0001);
            assert_eq!(result.used_points, points);
        } else {
            panic!("expected line")
        }
    }
}
