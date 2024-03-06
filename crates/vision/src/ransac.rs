use coordinate_systems::Framed;
use nalgebra::Point2;
use ordered_float::NotNan;
use rand::{rngs::StdRng, seq::SliceRandom, thread_rng, SeedableRng};
use types::line::{Line, Line2};

#[derive(Default, Debug, PartialEq)]
pub struct RansacResult<Frame> {
    pub line: Option<Line2<Frame>>,
    pub used_points: Vec<Framed<Frame, Point2<f32>>>,
}

pub struct Ransac<Frame> {
    pub unused_points: Vec<Framed<Frame, Point2<f32>>>,
    random_number_generator: StdRng,
}

impl<Frame> Ransac<Frame> {
    pub fn new(unused_points: Vec<Framed<Frame, Point2<f32>>>) -> Self {
        Self {
            unused_points,
            random_number_generator: StdRng::from_rng(thread_rng())
                .expect("Failed to create random number generator"),
        }
    }
}

impl<Frame> Ransac<Frame> {
    pub fn next_line(
        &mut self,
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
                    .choose_multiple(&mut self.random_number_generator, 2);
                let line = Line(*points.next().unwrap(), *points.next().unwrap());
                let score: f32 = self
                    .unused_points
                    .iter()
                    .filter(|&point| {
                        line.squared_distance_to_point(*point) <= maximum_score_distance_squared
                    })
                    .map(|point| 1.0 - line.distance_to_point(*point) / maximum_score_distance)
                    .sum();
                (line, score)
            })
            .max_by_key(|(_line, score)| NotNan::new(*score).expect("score should never be NaN"))
            .expect("max_by_key erroneously returned no result")
            .0;
        let (used_points, unused_points) = self.unused_points.iter().partition(|point| {
            best_line.squared_distance_to_point(**point) <= maximum_inclusion_distance_squared
        });
        self.unused_points = unused_points;
        RansacResult {
            line: Some(best_line),
            used_points,
        }
    }
}

#[cfg(test)]
mod test {
    use approx::assert_relative_eq;
    use coordinate_systems::IntoFramed;
    use nalgebra::point;

    use super::*;

    #[derive(Debug, PartialEq, Eq, Default)]
    struct SomeFrame;

    fn ransac_with_seed(
        unused_points: Vec<Framed<SomeFrame, Point2<f32>>>,
        seed: u64,
    ) -> Ransac<SomeFrame> {
        Ransac {
            unused_points,
            random_number_generator: StdRng::seed_from_u64(seed),
        }
    }

    #[test]
    fn ransac_empty_input() {
        let mut ransac = ransac_with_seed(vec![], 0);
        assert_eq!(ransac.next_line(10, 5.0, 5.0), RansacResult::default());
    }

    #[test]
    fn ransac_single_point() {
        let mut ransac = ransac_with_seed(vec![point![15.0, 15.0].framed()], 0);
        assert_eq!(ransac.next_line(10, 5.0, 5.0), RansacResult::default());
    }

    #[test]
    fn ransac_two_points() {
        let mut ransac = ransac_with_seed(
            vec![point![15.0, 15.0].framed(), point![30.0, 30.0].framed()],
            0,
        );
        let result = ransac.next_line(10, 5.0, 5.0);
        assert_relative_eq!(
            result.line.expect("No line found"),
            Line(point![15.0, 15.0].framed(), point![30.0, 30.0].framed())
        );
        assert_relative_eq!(result.used_points[0], point![15.0, 15.0].framed());
        assert_relative_eq!(result.used_points[1], point![30.0, 30.0].framed());
    }

    #[test]
    fn ransac_perfect_line() {
        let slope = 5.3;
        let y_intercept = -83.1;
        let points: Vec<_> = (0..100)
            .map(|x| point![x as f32, y_intercept + x as f32 * slope].framed())
            .collect();

        let mut ransac = ransac_with_seed(points.clone(), 0);
        let result = ransac.next_line(15, 1.0, 1.0);
        let line = result.line.expect("No line was found");
        assert_relative_eq!(line.slope(), slope, epsilon = 0.0001);
        assert_relative_eq!(line.y_axis_intercept(), y_intercept, epsilon = 0.0001);
        assert_eq!(result.used_points, points);
    }
}
