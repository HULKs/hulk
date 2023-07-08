use geometry::{
    line::{Line, Line2},
    Distance,
};
use linear_algebra::Point2;
use ordered_float::NotNan;
use rand::{rngs::StdRng, seq::SliceRandom, thread_rng, Rng, SeedableRng};
pub mod circles;

#[derive(Default, Debug, PartialEq)]
pub struct RansacResult<Frame> {
    pub line: Option<Line2<Frame>>,
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
    pub fn next_line(
        &mut self,
        random_number_generator: &mut impl Rng,
        iterations: usize,
        maximum_score_distance: f32,
        maximum_inclusion_distance: f32,
    ) -> RansacResult<Frame> {
        if self.unused_points.len() < 2 {
            return RansacResult {
                line: None,
                used_points: vec![],
            };
        }

        let maximum_score_distance_squared = maximum_score_distance * maximum_score_distance;
        let maximum_inclusion_distance_squared =
            maximum_inclusion_distance * maximum_inclusion_distance;
        let best_line = (0..iterations)
            .map(|_| {
                let mut points = self
                    .unused_points
                    .choose_multiple(random_number_generator, 2);
                let line = Line::from_points(*points.next().unwrap(), *points.next().unwrap());
                let score: f32 = self
                    .unused_points
                    .iter()
                    .filter(|&point| {
                        line.squared_distance_to(*point) <= maximum_score_distance_squared
                    })
                    .map(|point| 1.0 - line.distance_to(*point) / maximum_score_distance)
                    .sum();
                (line, score)
            })
            .max_by_key(|(_line, score)| NotNan::new(*score).expect("score should never be NaN"))
            .expect("max_by_key erroneously returned no result")
            .0;
        let (used_points, unused_points) = self.unused_points.iter().partition(|point| {
            best_line.squared_distance_to(**point) <= maximum_inclusion_distance_squared
        });
        self.unused_points = unused_points;
        RansacResult {
            line: Some(best_line),
            used_points,
        }
    }
}

pub struct ClusteringRansac<Frame> {
    pub unused_points: Vec<Point2<Frame, f32>>,
    random_number_generator: StdRng,
}

impl<Frame> ClusteringRansac<Frame> {
    pub fn new(unused_points: Vec<Point2<Frame, f32>>) -> Self {
        Self {
            unused_points,
            random_number_generator: StdRng::from_rng(thread_rng())
                .expect("Failed to create random number generator"),
        }
    }

    pub fn next_line_cluster(
        &mut self,
        iterations: usize,
        maximum_distance: f32,
        maximum_gap: f32,
    ) -> Vec<Point2<Frame, f32>> {
        if self.unused_points.len() < 2 {
            return vec![];
        }
        let best_cluster = (0..iterations)
            .flat_map(|_| {
                let mut points = self
                    .unused_points
                    .choose_multiple(&mut self.random_number_generator, 2);
                let line = Line(*points.next().unwrap(), *points.next().unwrap());
                let (mut used_points, unused_points): (Vec<_>, Vec<_>) = self
                    .unused_points
                    .clone()
                    .into_iter()
                    .partition(|&point| line.distance_to_point(point) <= maximum_distance);
                let difference_on_line = line.1 - line.0;
                used_points.sort_by(|left, right| {
                    let difference_to_left = *left - line.0;
                    let difference_to_right = *right - line.0;
                    let left = difference_to_left.dot(difference_on_line)
                        / difference_on_line.norm_squared();
                    let right = difference_to_right.dot(difference_on_line)
                        / difference_on_line.norm_squared();
                    left.total_cmp(&right)
                });
                let mut clusters = vec![];
                while !used_points.is_empty() {
                    let split_index = (1..used_points.len())
                        .find(|&index| {
                            distance(used_points[index - 1], used_points[index]) > maximum_gap
                        })
                        .unwrap_or(used_points.len());
                    let after_gap = used_points.split_off(split_index);
                    let mut unused_points = unused_points.clone();
                    unused_points.extend(after_gap.iter());
                    let score = used_points.len();
                    clusters.push(ScoredCluster {
                        used_points,
                        unused_points,
                        score,
                    });
                    used_points = after_gap;
                }
                clusters
            })
            .max_by_key(|scored_line| scored_line.score)
            .expect("max_by_key erroneously returned no result");

        self.unused_points = best_cluster.unused_points;
        best_cluster.used_points
    }
}

struct ScoredCluster<Frame> {
    used_points: Vec<Point2<Frame, f32>>,
    unused_points: Vec<Point2<Frame, f32>>,
    score: usize,
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
            ransac.next_line(&mut rng, 10, 5.0, 5.0),
            RansacResult::default()
        );
    }

    #[test]
    fn ransac_single_point() {
        let mut ransac = Ransac::<SomeFrame>::new(vec![]);
        let mut rng = ChaChaRng::from_entropy();
        assert_eq!(
            ransac.next_line(&mut rng, 10, 5.0, 5.0),
            RansacResult::default()
        );
    }

    #[test]
    fn ransac_two_points() {
        let p1 = point![15.0, 15.0];
        let p2 = point![30.0, 30.0];
        let mut ransac = Ransac::<SomeFrame>::new(vec![p1, p2]);
        let mut rng = ChaChaRng::from_entropy();
        let RansacResult { line, used_points } = ransac.next_line(&mut rng, 10, 5.0, 5.0);
        let line = line.expect("No line found");
        println!("{line:#?}");
        println!("{used_points:#?}");

        assert!(
            relative_eq!(line, Line::from_points(p1, p2))
                || relative_eq!(line, Line::from_points(p2, p1))
        );
        assert!(relative_eq!(used_points[0], p1) || relative_eq!(used_points[0], p2));
        assert!(relative_eq!(used_points[1], p2) || relative_eq!(used_points[0], p1));
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
        let result = ransac.next_line(&mut rng, 15, 1.0, 1.0);
        let line = result.line.expect("No line was found");
        assert_relative_eq!(line.slope(), slope, epsilon = 0.0001);
        assert_relative_eq!(line.y_axis_intercept(), y_intercept, epsilon = 0.0001);
        assert_eq!(result.used_points, points);
    }
}
