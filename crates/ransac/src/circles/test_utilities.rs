use itertools::Itertools;
use nalgebra::RealField;
use rand::{
    distributions::{Distribution, Uniform},
    rngs::StdRng,
    SeedableRng,
};

use linear_algebra::{point, Point2};

#[allow(dead_code)]
pub fn generate_circle<Frame, T>(
    circle_center: &Point2<Frame, T>,
    point_count: usize,
    circle_radius: T,
    circle_radius_variance: T,
    random_seed: u64,
) -> Vec<Point2<Frame, T>>
where
    T: Copy + RealField + rand::distributions::uniform::SampleUniform,
{
    let angle_range = Uniform::from(-T::pi()..T::pi());

    let random_number_generator = StdRng::seed_from_u64(random_seed);

    let randomized_angles_iter = angle_range
        .sample_iter(random_number_generator.clone())
        .take(point_count);

    let randomized_radiuses = if circle_radius_variance.abs() <= T::default_epsilon() {
        vec![circle_radius; point_count]
    } else {
        let radius_range = Uniform::from(
            (circle_radius - circle_radius_variance)..(circle_radius + circle_radius_variance),
        );

        radius_range
            .sample_iter(random_number_generator)
            .take(point_count)
            .collect_vec()
    };

    let circle_points_iter =
        randomized_angles_iter
            .zip(randomized_radiuses.iter())
            .map(|(angle, radius)| {
                point![
                    (angle.cos() * *radius) + circle_center.x(),
                    (angle.sin() * *radius) + circle_center.y()
                ]
            });

    let out_vec = circle_points_iter.collect_vec();

    for point in &out_vec {
        let percieved_radius = (*circle_center - *point).norm();
        assert!(
            (percieved_radius - circle_radius).abs()
                <= circle_radius_variance + T::from_f64(1e-5).unwrap()
        );
    }

    out_vec
}
