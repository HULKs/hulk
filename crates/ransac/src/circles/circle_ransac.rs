use itertools::Itertools;
use linear_algebra::{point, Point2};
use nalgebra::ComplexField;
use rand::{rngs::StdRng, seq::SliceRandom, thread_rng, SeedableRng};

use super::circle_fitting_model::{circle_fit_radius_inlier_mask, GenericCircle};

#[derive(Default, Debug, PartialEq)]
pub struct RansacResultCircle<Frame, T = f32>
where
    T: ComplexField + Clone + Copy,
{
    pub output: Option<super::circle_fitting_model::GenericCircle<Frame, T>>,
    pub used_points: Vec<Point2<Frame, T>>,
}

pub struct RansacCircleWithRadius<Frame, T>
where
    T: ComplexField + Clone + Copy,
{
    radius: T,
    pub unused_points: Vec<Point2<Frame, T>>,
    random_number_generator: StdRng,
}

impl<Frame, T> RansacCircleWithRadius<Frame, T>
where
    T: ComplexField + Clone + Copy,
{
    pub fn new(radius: T, unused_points: Vec<Point2<Frame, T>>) -> Self {
        Self {
            radius,
            unused_points,
            random_number_generator: StdRng::from_rng(thread_rng())
                .expect("Failed to create random number generator"),
        }
    }
}

impl<Frame, T> RansacCircleWithRadius<Frame, T>
where
    T: ComplexField<RealField = T> + Clone + Copy + std::cmp::PartialOrd,
{
    pub fn next_candidate(
        &mut self,
        iterations: usize,
        radius_variance: T,
    ) -> RansacResultCircle<Frame, T> {
        if self.unused_points.len() < 2 {
            return RansacResultCircle::<Frame, T> {
                output: None,
                used_points: vec![],
            };
        }

        let radius = self.radius;

        // Define the minimum distance the furthest two points should take (corresponds to minimum arc length resulted by the 3 points)
        // tuning_factor should be >= 1 to have useful results
        // TODO make this configurable
        let tuning_factor = T::from_f64(1.2).unwrap();
        // TODO Take this from field dimensions or otherwise!
        let w = T::from_f64(0.05).unwrap(); // line width

        // If a triangle is constructed from the 3 points, the longest side should be at least as long as this.
        let minimum_furthest_points_distance = (radius.powi(2) - (radius - w).powi(2)).sqrt()
            * T::from_f64(2.0).unwrap()
            * tuning_factor;

        let best_candidate_model_option = (0..iterations)
            .filter_map(|_| {
                let three_points = self
                    .unused_points
                    .choose_multiple(&mut self.random_number_generator, 3)
                    .collect_vec();

                let ab = (three_points[0].coords() - three_points[1].coords()).norm();
                let bc = (three_points[1].coords() - three_points[2].coords()).norm();
                let ca = (three_points[0].coords() - three_points[2].coords()).norm();

                if ab < minimum_furthest_points_distance
                    && bc < minimum_furthest_points_distance
                    && ca < minimum_furthest_points_distance
                {
                    return None;
                }
                let candidate_circle = Self::circle_from_three_points(
                    three_points[0],
                    three_points[1],
                    three_points[2],
                );
                // If the candidate radius isn't within 30% of the expected radius, this is bad!
                if candidate_circle.radius - radius > radius * T::from_f64(0.3).unwrap() {
                    return None;
                }

                let inlier_points_mask = circle_fit_radius_inlier_mask(
                    &candidate_circle,
                    &self.unused_points,
                    radius_variance,
                );

                Some((candidate_circle, inlier_points_mask))
            })
            .max_by_key(|scored_circle| {
                scored_circle
                    .1
                    .as_slice()
                    .into_iter()
                    .filter(|is_inlier| **is_inlier)
                    .count()
            });

        let (candidate_circle, used_points) =
            if let Some((candidate_circle, inliers_mask)) = best_candidate_model_option {
                let (used_points, unused_points) = inliers_mask
                    .into_iter()
                    .zip(&self.unused_points)
                    .partition_map(|(is_inlier, point)| {
                        if is_inlier {
                            itertools::Either::Left(point)
                        } else {
                            itertools::Either::Right(point)
                        }
                    });

                self.unused_points = unused_points;

                (Some(candidate_circle), used_points)
            } else {
                (None, vec![])
            };

        RansacResultCircle::<Frame, T> {
            output: candidate_circle,
            used_points,
        }
    }

    fn circle_from_three_points(
        a: &Point2<Frame, T>,
        b: &Point2<Frame, T>,
        c: &Point2<Frame, T>,
    ) -> GenericCircle<Frame, T> {
        let two_t = T::from_f64(2.0).unwrap();

        // Let points be a, b, c
        let ba_diff = b.coords() - a.coords();
        let cb_diff = c.coords() - b.coords();
        let ab_mid = (a.coords() + b.coords()) / two_t;
        let bc_mid = (b.coords() + c.coords()) / two_t;

        let ab_perpendicular_slope = -(ba_diff.x() / ba_diff.y());
        let bc_perpendicular_slope = -(cb_diff.x() / cb_diff.y());

        // using y - y1 = m (x - x1) form, get x and y where centre is intersection of ab & bc perpendicular bisectors!
        let centre_x = ((bc_mid.y() - ab_mid.y()) + (ab_perpendicular_slope * ab_mid.x())
            - (bc_perpendicular_slope * bc_mid.x()))
            / (ab_perpendicular_slope - bc_perpendicular_slope);

        let centre_y = ab_perpendicular_slope * (centre_x - ab_mid.x()) + ab_mid.y();
        let centre = point![centre_x, centre_y];
        let radius = (a.coords() - centre.coords()).norm();

        GenericCircle { centre, radius }
    }
}

#[cfg(test)]
mod test {

    use approx::assert_relative_eq;
    use linear_algebra::{point, Point2};
    use rand::{rngs::StdRng, SeedableRng};

    use crate::circles::{circle_ransac::RansacResultCircle, test_utilities::generate_circle};

    use super::RansacCircleWithRadius;

    type T = f32;

    const TYPICAL_RADIUS: T = 0.75;

    const REL_ASSERT_EPSILON: T = 1e-5;

    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    struct SomeFrame;

    fn ransac_circle_with_seed(
        unused_points: Vec<Point2<SomeFrame>>,
        seed: u64,
        radius: T,
    ) -> RansacCircleWithRadius<SomeFrame, T> {
        RansacCircleWithRadius::<SomeFrame, T> {
            radius,
            unused_points,
            random_number_generator: StdRng::seed_from_u64(seed),
        }
    }

    #[test]
    fn ransac_empty_input() {
        let mut ransac = ransac_circle_with_seed(vec![], 0, TYPICAL_RADIUS);
        assert_eq!(
            ransac.next_candidate(10, 5.0),
            RansacResultCircle::<SomeFrame>::default()
        );
    }

    #[test]
    fn ransac_single_point() {
        let mut ransac = ransac_circle_with_seed(vec![point![5.0, 5.0]], 0, TYPICAL_RADIUS);
        assert_eq!(
            ransac.next_candidate(10, 5.0),
            RansacResultCircle::<SomeFrame>::default()
        );
    }

    #[test]
    fn three_point_circle_equation_test() {
        let centre = point![2.0, 1.5];
        let radius = TYPICAL_RADIUS;
        let angles = [10.0, 45.0, 240.0];

        let points: Vec<_> = angles
            .iter()
            .map(|a: &T| point![radius * a.cos() + centre.x(), radius * a.sin() + centre.y()])
            .collect();

        let circle = RansacCircleWithRadius::<SomeFrame, T>::circle_from_three_points(
            &points[0], &points[1], &points[2],
        );
        assert_relative_eq!(circle.centre, centre, epsilon = REL_ASSERT_EPSILON);
    }

    #[test]
    fn ransac_circle_three_points() {
        let centre = point![2.0, 1.5];
        let radius = TYPICAL_RADIUS;
        let angles = [10.0, 45.0, 240.0];

        let points: Vec<_> = angles
            .iter()
            .map(|a: &T| point![radius * a.cos() + centre.x(), radius * a.sin() + centre.y()])
            .collect();

        let mut ransac = ransac_circle_with_seed(points.clone(), 0, TYPICAL_RADIUS);
        let result = ransac.next_candidate(10, 5.0);

        let out_circle = result.output.expect("No circle found");

        assert_eq!(points.len(), result.used_points.len());
        assert_relative_eq!(out_circle.centre, centre, epsilon = REL_ASSERT_EPSILON);
        assert_relative_eq!(
            out_circle.radius,
            TYPICAL_RADIUS,
            epsilon = REL_ASSERT_EPSILON
        );
        assert_relative_eq!(result.used_points[0], points[0]);
        assert_relative_eq!(result.used_points[1], points[1]);
    }

    #[test]
    fn ransac_perfect_circle() {
        let centre = point![2.0, 1.5];
        let radius = TYPICAL_RADIUS;
        let points: Vec<Point2<SomeFrame>> = generate_circle(&centre, 100, radius, 0.0, 0);

        let mut ransac = ransac_circle_with_seed(points.clone(), 0, TYPICAL_RADIUS);
        let result = ransac.next_candidate(15, 0.1);
        let output = result.output.expect("No circle was found");
        assert_relative_eq!(output.centre, centre, epsilon = 0.0001);
        assert_relative_eq!(output.radius, radius, epsilon = 0.0001);
        assert_eq!(result.used_points, points);
    }
}
