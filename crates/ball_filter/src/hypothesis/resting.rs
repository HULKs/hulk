use coordinate_systems::Ground;
use filtering::kalman_filter::KalmanFilter;
use linear_algebra::{Isometry2, Point2};
use nalgebra::Matrix2;
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

pub(super) trait RestingPredict {
    fn predict(
        &mut self,
        last_to_current_odometry: Isometry2<Ground, Ground>,
        process_noise: Matrix2<f32>,
    );
}

pub(super) trait RestingUpdate {
    fn update(&mut self, measurement: Point2<Ground>, noise: Matrix2<f32>);
}

impl RestingPredict for MultivariateNormalDistribution<2> {
    fn predict(
        &mut self,
        last_to_current_odometry: Isometry2<Ground, Ground>,
        process_noise: Matrix2<f32>,
    ) {
        let rotation = last_to_current_odometry.inner.rotation.to_rotation_matrix();
        let translation = last_to_current_odometry.inner.translation.vector;

        KalmanFilter::predict(
            self,
            *rotation.matrix(),
            Matrix2::identity(),
            translation,
            process_noise,
        );
    }
}

impl RestingUpdate for MultivariateNormalDistribution<2> {
    fn update(&mut self, measurement: Point2<Ground>, noise: Matrix2<f32>) {
        KalmanFilter::update(self, Matrix2::identity(), measurement.inner.coords, noise)
    }
}
