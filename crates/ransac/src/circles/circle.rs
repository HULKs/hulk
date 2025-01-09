use std::f32::consts::FRAC_PI_4;

use itertools::Itertools;
use ordered_float::NotNan;
use rand::{seq::SliceRandom, Rng};

use geometry::circle::Circle;
use linear_algebra::{point, Point2};

struct Parameters {
    radius: f32,
    inlier_threshold_on_residual: f32,
    minimum_furthest_points_distance_squared: f32,
    average_point_fitting_score: f32,
    sample_size_fraction: f32,
}

#[derive(Default, Debug, PartialEq)]
pub struct RansacResultCircle<Frame> {
    pub circle: Circle<Frame>,
    pub used_points: Vec<Point2<Frame>>,
    pub score: f32,
    pub total_iterations: usize,
}

/// This method allows to find circles by transforming to another frame (i.e. Ground).
/// Otherwise, an ellipse fitting method should be used.
pub struct RansacCircleWithTransformation<OriginalFrame, SearchFrame> {
    parameters: Parameters,
    pub unused_points_original: Vec<Point2<OriginalFrame>>,
    unused_points_transformed: Vec<Point2<SearchFrame>>,
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct RansacResultCircleWithTransformation<OriginalFrame, SearchFrame> {
    pub circle: Circle<SearchFrame>,
    pub used_points_original: Vec<Point2<OriginalFrame>>,
    pub used_points_transformed: Vec<Point2<SearchFrame>>,
    pub score: f32,
    pub total_iterations: usize,
}

impl<OriginalFrame, SearchFrame> RansacCircleWithTransformation<OriginalFrame, SearchFrame> {
    pub fn new(
        radius: f32,
        accepted_radius_variance: f32,
        points: Vec<Point2<OriginalFrame>>,
        transformer_function: impl Fn(&Point2<OriginalFrame>) -> Option<Point2<SearchFrame>>,
        early_exit_average_point_fitting_score: Option<f32>,
        sample_size_fraction: Option<f32>,
    ) -> Self {
        const MINIMUM_ANGLE_OF_ARC: f32 = FRAC_PI_4;
        let minimum_furthest_points_distance =
            compute_minimum_point_distance(MINIMUM_ANGLE_OF_ARC, radius);

        let (unused_points_original, unused_points_transformed) = points
            .iter()
            .filter_map(|point| {
                let output = transformer_function(point);
                output.map(|transformed| (point, transformed))
            })
            .unzip();

        Self {
            unused_points_original,
            unused_points_transformed,

            parameters: Parameters {
                radius,
                inlier_threshold_on_residual: accepted_radius_variance,
                minimum_furthest_points_distance_squared: minimum_furthest_points_distance.powi(2),
                average_point_fitting_score: early_exit_average_point_fitting_score
                    .unwrap_or(1.1)
                    .clamp(0.0, 1.0),
                sample_size_fraction: sample_size_fraction.unwrap_or(0.35),
            },
        }
    }

    pub fn next_candidate(
        &mut self,
        random_number_generator: &mut impl Rng,
        iterations: usize,
    ) -> Option<RansacResultCircleWithTransformation<OriginalFrame, SearchFrame>> {
        let best_candidate_model_option = get_best_candidate::<SearchFrame>(
            &self.unused_points_transformed,
            iterations,
            &self.parameters,
            random_number_generator,
        );
        best_candidate_model_option.map(
            |(candidate_circle, inliers_mask, score, total_iterations)| {
                let (used_points_transformed, unused_points_transformed) = inliers_mask
                    .iter()
                    .zip(&self.unused_points_transformed)
                    .partition_map(|(&is_inlier, point)| {
                        if is_inlier {
                            itertools::Either::Left(point)
                        } else {
                            itertools::Either::Right(point)
                        }
                    });

                let (used_points_original, unused_points_original) = inliers_mask
                    .into_iter()
                    .zip(&self.unused_points_original)
                    .partition_map(|(is_inlier, point)| {
                        if is_inlier {
                            itertools::Either::Left(point)
                        } else {
                            itertools::Either::Right(point)
                        }
                    });

                self.unused_points_original = unused_points_original;
                self.unused_points_transformed = unused_points_transformed;

                RansacResultCircleWithTransformation {
                    circle: candidate_circle,
                    used_points_original,
                    used_points_transformed,
                    score,
                    total_iterations,
                }
            },
        )
    }

    pub fn get_unused_points_original<'a>(&'a self) -> &'a [Point2<OriginalFrame>] {
        &self.unused_points_original
    }

    pub fn get_unused_points_transformed<'a>(&'a self) -> &'a [Point2<SearchFrame>] {
        &self.unused_points_transformed
    }
}

fn get_best_candidate<Frame>(
    src_unused_points: &[Point2<Frame>],
    iterations: usize,
    parameters: &Parameters,
    random_number_generator: &mut impl Rng,
) -> Option<(Circle<Frame>, Vec<bool>, f32, usize)> {
    let src_point_count = src_unused_points.len();
    if src_point_count < 3 {
        return None;
    }

    let sampled_population_size = {
        let min_sampled_population_size = 150;
        let candidate_sample_size =
            src_unused_points.len() as f32 * parameters.sample_size_fraction;
        if candidate_sample_size > min_sampled_population_size as f32 {
            candidate_sample_size as usize
        } else {
            src_point_count
        }
    };

    let radius_squared = parameters.radius.powi(2);

    // average_point_fitting_score

    let chunk_count = 10;
    let iter_chunk_size = (iterations / chunk_count).max(100);
    let mut best: Option<(Circle<Frame>, f32)> = None;
    let mut total_iterations = iter_chunk_size * chunk_count;

    let unused_points = src_unused_points
        .choose_multiple(random_number_generator, sampled_population_size)
        .collect_vec();

    for chunk in 0..chunk_count {
        let new_best = (0..iter_chunk_size)
            .filter_map(|_| {
                let (point1, point2, point3) = unused_points
                    .choose_multiple(random_number_generator, 3)
                    .cloned()
                    .collect_tuple()
                    .unwrap();

                let ab_squared = (*point1 - *point2).norm_squared();
                let bc_squared = (*point2 - *point3).norm_squared();
                let ca_squared = (*point3 - *point1).norm_squared();
                if ab_squared < parameters.minimum_furthest_points_distance_squared
                    && bc_squared < parameters.minimum_furthest_points_distance_squared
                    && ca_squared < parameters.minimum_furthest_points_distance_squared
                {
                    return None;
                }
                let candidate_circle = circle_from_three_points(point1, point2, point3);
                let initial_max_variance = 1.5 * parameters.inlier_threshold_on_residual;
                if (candidate_circle.radius - parameters.radius).powi(2) > initial_max_variance {
                    return None;
                }

                let score = unused_points
                    .iter()
                    .filter_map(|&&point| {
                        let distance_squared = (point - candidate_circle.center).norm_squared();
                        let residual_abs = (distance_squared - radius_squared).abs();
                        let is_inlier = residual_abs <= parameters.inlier_threshold_on_residual;

                        if is_inlier {
                            let cost = residual_abs / parameters.inlier_threshold_on_residual;
                            assert!(
                                cost <= 1.0,
                                "The cost MUST be less than one but it is {}",
                                cost
                            );
                            Some(1.0 - cost)
                        } else {
                            None
                        }
                    })
                    .sum::<f32>();

                Some((candidate_circle, score))
            })
            .max_by_key(|scored_circle| NotNan::new(scored_circle.1).unwrap_or_default());

        // Any better way to write this?

        match (&best, &new_best) {
            (None, Some(_)) => best = new_best,
            (Some((_, current_best_score)), Some((_, new_score))) => {
                if new_score > current_best_score {
                    let best_score = *new_score;
                    best = new_best;
                    if best_score / sampled_population_size as f32
                        > parameters.average_point_fitting_score
                    {
                        total_iterations = chunk * iter_chunk_size;
                        break;
                    }
                }
            }
            _ => (),
        }
    }
    best.map(|(circle, _score)| {
        let mut total_score = 0.0;
        let center = circle.center;

        let inlier_points_mask = src_unused_points
            .iter()
            .map(|&point| {
                let distance_squared = (point - center).norm_squared();
                let residual_abs = (distance_squared - radius_squared).abs();
                let is_inlier = residual_abs <= parameters.inlier_threshold_on_residual;
                if is_inlier {
                    let cost = residual_abs / parameters.inlier_threshold_on_residual;
                    assert!(
                        cost <= 1.0,
                        "The cost MUST be less than one but it is {}",
                        cost
                    );
                    total_score += 1.0 - cost
                }
                is_inlier
            })
            .collect_vec();

        let average_score = total_score / src_point_count as f32;
        assert!(
            average_score <= 1.0,
            "The average score MUST be less than one but it is {}",
            average_score
        );

        (circle, inlier_points_mask, average_score, total_iterations)
    })
}

fn circle_from_three_points<Frame>(
    a: &Point2<Frame>,
    b: &Point2<Frame>,
    c: &Point2<Frame>,
) -> Circle<Frame> {
    let ba_diff = *b - *a;
    let cb_diff = *c - *b;
    let ab_mid = (a.coords() + b.coords()) / 2.0;
    let bc_mid = (b.coords() + c.coords()) / 2.0;

    let ab_perpendicular_slope = -(ba_diff.x() / ba_diff.y());
    let bc_perpendicular_slope = -(cb_diff.x() / cb_diff.y());

    // Center is the intersection point of perpendicular bisectors of lines ab, bc lines.
    let center_x = ((bc_mid.y() - ab_mid.y()) + (ab_perpendicular_slope * ab_mid.x())
        - (bc_perpendicular_slope * bc_mid.x()))
        / (ab_perpendicular_slope - bc_perpendicular_slope);

    let center_y = ab_perpendicular_slope * (center_x - ab_mid.x()) + ab_mid.y();
    let center = point![center_x, center_y];
    let radius = (a.coords() - center.coords()).norm();

    Circle { center, radius }
}

/// Calculates distance between two points a, b on a circle,
/// based on the angle `alpha` (`angle_at_center_to_points`) between ac & cb lines where c is the center.
/// For radius r, length of line ab = sin(alpha/2) * 2 * r
fn compute_minimum_point_distance(angle_at_center_to_points: f32, radius: f32) -> f32 {
    (angle_at_center_to_points / 2.0).sin() * 2.0 * radius
}

#[cfg(test)]
mod test {

    use crate::circles::{
        circle::{circle_from_three_points, RansacCircleWithTransformation},
        test_utilities::generate_circle,
    };
    use approx::assert_relative_eq;
    use linear_algebra::{point, Point2};
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;

    const TYPICAL_RADIUS: f32 = 0.75;
    const ACCEPTED_RADIUS_VARIANCE: f32 = 0.1;
    const REL_ASSERT_EPSILON: f32 = 1e-5;

    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    struct SomeFrame;
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    struct OtherFrame;

    fn _some_to_other(v: &Point2<SomeFrame>) -> Option<Point2<OtherFrame>> {
        Some(point![v.x(), v.y()])
    }

    #[test]
    fn ransac_empty_input() {
        let mut rng = ChaChaRng::from_entropy();
        let mut ransac = RansacCircleWithTransformation::<SomeFrame, OtherFrame>::new(
            TYPICAL_RADIUS,
            ACCEPTED_RADIUS_VARIANCE,
            vec![],
            _some_to_other,
            None,
            Some(0.35),
        );
        assert_eq!(ransac.next_candidate(&mut rng, 10), None);
    }

    #[test]
    fn ransac_single_point() {
        let mut rng = ChaChaRng::from_entropy();
        let mut ransac = RansacCircleWithTransformation::<SomeFrame, OtherFrame>::new(
            TYPICAL_RADIUS,
            ACCEPTED_RADIUS_VARIANCE,
            vec![point![5.0, 5.0]],
            _some_to_other,
            None,
            Some(0.35),
        );
        assert_eq!(ransac.next_candidate(&mut rng, 10), None);
    }

    #[test]
    fn three_point_circle_equation_test() {
        let center = point![2.0, 1.5];
        let radius = TYPICAL_RADIUS;
        let angles = [10.0, 45.0, 240.0];

        let points: Vec<_> = angles
            .iter()
            .map(|a: &f32| point![radius * a.cos() + center.x(), radius * a.sin() + center.y()])
            .collect();

        let circle = circle_from_three_points::<SomeFrame>(&points[0], &points[1], &points[2]);
        assert_relative_eq!(circle.center, center, epsilon = REL_ASSERT_EPSILON);
    }

    #[test]
    fn ransac_circle_three_points() {
        let center = point![2.0, 1.5];
        let radius = TYPICAL_RADIUS;
        let angles = [10.0, 45.0, 240.0];

        let points: Vec<_> = angles
            .iter()
            .map(|a: &f32| point![radius * a.cos() + center.x(), radius * a.sin() + center.y()])
            .collect();
        let mut rng = ChaChaRng::from_entropy();
        let mut ransac = RansacCircleWithTransformation::<SomeFrame, OtherFrame>::new(
            TYPICAL_RADIUS,
            ACCEPTED_RADIUS_VARIANCE,
            points.clone(),
            _some_to_other,
            None,
            Some(0.35),
        );
        let result = ransac
            .next_candidate(&mut rng, 10)
            .expect("No circle found");

        let detected_circle = result.circle;

        assert_eq!(points.len(), result.used_points_original.len());
        assert_relative_eq!(detected_circle.center, center, epsilon = REL_ASSERT_EPSILON);
        assert_relative_eq!(
            detected_circle.radius,
            TYPICAL_RADIUS,
            epsilon = REL_ASSERT_EPSILON
        );

        assert_relative_eq!(result.used_points_original[0], points[0]);
        assert_relative_eq!(result.used_points_original[1], points[1]);
    }

    #[test]
    fn ransac_perfect_circle() {
        let center = point![2.0, 1.5];
        let radius = TYPICAL_RADIUS;
        let points: Vec<Point2<SomeFrame>> = generate_circle(&center, 100, radius, 0.0, 0);
        let mut rng = ChaChaRng::from_entropy();
        let mut ransac = RansacCircleWithTransformation::<SomeFrame, OtherFrame>::new(
            TYPICAL_RADIUS,
            ACCEPTED_RADIUS_VARIANCE,
            points.clone(),
            _some_to_other,
            None,
            None,
        );
        let result = ransac
            .next_candidate(&mut rng, 15)
            .expect("No circle was found");
        let detected_circle = result.circle;
        assert_relative_eq!(
            detected_circle.center,
            _some_to_other(&center).unwrap(),
            epsilon = 0.0001
        );
        assert_relative_eq!(detected_circle.radius, radius, epsilon = 0.0001);
        assert_eq!(result.used_points_original, points);
    }
}
