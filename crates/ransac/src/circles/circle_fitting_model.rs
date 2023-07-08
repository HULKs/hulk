use geometry::circle::Circle;
use itertools::Itertools;
use linear_algebra::Point2;
use nalgebra::{ComplexField, Dyn, OVector};

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct GenericCircle<Frame, T = f32>
where
    T: ComplexField + Clone + Copy,
{
    pub radius: T,
    pub centre: Point2<Frame, T>,
}

impl<Frame, T> From<GenericCircle<Frame, T>> for Circle<Frame>
where
    T: Copy + Clone + ComplexField,
    Point2<Frame, f32>: From<Point2<Frame, T>>,
    f32: From<T>,
{
    fn from(value: GenericCircle<Frame, T>) -> Self {
        Circle {
            center: value.centre.into(),
            radius: value.radius.into(),
        }
    }
}

impl<Frame, T> From<Circle<Frame>> for GenericCircle<Frame, T>
where
    T: Copy + Clone + ComplexField,
    Point2<Frame, T>: From<Point2<Frame, f32>>,
    T: From<f32>,
{
    fn from(value: Circle<Frame>) -> Self {
        GenericCircle::<Frame, T> {
            centre: value.center.into(),
            radius: value.radius.into(),
        }
    }
}

fn circle_fit_radius_residual_internal<Frame, T>(
    candidate_circle: &GenericCircle<Frame, T>,
    points: &[Point2<Frame, T>],
) -> OVector<T, Dyn>
where
    T: ComplexField<RealField = T> + Copy + Clone + std::cmp::PartialOrd,
{
    OVector::<T, Dyn>::from_iterator(
        points.len(),
        points.iter().map(|point| {
            let difference = (*point - candidate_circle.centre).norm() - candidate_circle.radius;
            difference
        }),
    )
}

pub fn get_inlier_count<T>(radius_based_residuals: &[T], radius_variance: T) -> usize
where
    T: ComplexField<RealField = T> + Copy + Clone + std::cmp::PartialOrd,
{
    radius_based_residuals
        .iter()
        .filter(|residual| is_inlier::<T>(**residual, radius_variance))
        .count()
}

pub fn circle_fit_radius_inlier_mask<Frame, T>(
    candidate_circle: &GenericCircle<Frame, T>,
    points: &[Point2<Frame, T>],
    radius_variance: T,
) -> Vec<bool>
where
    T: ComplexField<RealField = T> + Copy + Clone + std::cmp::PartialOrd,
{
    // TODO Find out why this iter chain is such a mess...
    circle_fit_radius_residual_internal(candidate_circle, points)
        .map(|residual| is_inlier(residual, radius_variance))
        .as_slice()
        .into_iter()
        .map(|x| *x)
        .collect_vec()
}

#[inline]
pub fn is_inlier<T>(single_residual_value: T, radius_variance: T) -> bool
where
    T: ComplexField<RealField = T> + std::cmp::PartialOrd,
{
    single_residual_value.abs() <= radius_variance
}

#[cfg(test)]
mod tests {
    use crate::circles::{
        circle_fitting_model::{
            circle_fit_radius_residual_internal, get_inlier_count, GenericCircle,
        },
        test_utilities::generate_circle,
    };

    use linear_algebra::point;

    type T = f32;
    const RADIUS: T = 0.75;
    const SEED: u64 = 0;

    #[test]
    fn test_residual_calculation() {
        const POINT_COUNT: usize = 20;
        let radius_variance = 0.0;

        let candidate_circle = GenericCircle::<T> {
            radius: RADIUS,
            centre: point![2.0, 4.0],
        };

        let circle_points_ground = generate_circle(
            &candidate_circle.centre,
            POINT_COUNT,
            candidate_circle.radius,
            radius_variance,
            SEED,
        );

        // Since we give points fitting perfectly to a circle of the given radius, this residual should be nearly zero!
        let residual =
            circle_fit_radius_residual_internal(&candidate_circle, &circle_points_ground);

        println!(
            "Circle centre: {:?}, circle radius: {:?} circle points: {:?}",
            candidate_circle.centre, candidate_circle.radius, circle_points_ground
        );
        println!("Residual -> should be close to 0 {:?}", residual);
        assert!(residual.norm() < 1e-6);
    }

    #[test]
    fn test_residual_calculation_with_variance() {
        const POINT_COUNT: usize = 20;
        let radius_variance = 0.1;

        let candidate_circle = GenericCircle::<T> {
            centre: point![2.0, 4.0],
            radius: RADIUS,
        };

        let circle_points_ground = generate_circle(
            &candidate_circle.centre,
            POINT_COUNT,
            candidate_circle.radius,
            radius_variance,
            SEED,
        );

        let residuals =
            circle_fit_radius_residual_internal(&candidate_circle, &circle_points_ground);

        println!(
            "Circle centre: {:?}, circle radius: {:?} circle points: {:?}",
            candidate_circle.centre, candidate_circle.radius, circle_points_ground
        );
        println!(
            "Residuals should be within {:?}  residual {:?}",
            radius_variance, residuals
        );

        // Residual is square of the distance difference!

        let count = get_inlier_count::<T>(residuals.as_slice(), radius_variance);

        println!(
            "Residual and inlier count: {:?}, {:?}, {:?}%",
            count,
            radius_variance,
            (count as T / residuals.len() as T * 100.0)
        );

        assert_eq!(count, residuals.len());

        // assert!(residual.norm() < 1e-6);
    }
}
