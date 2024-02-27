use std::{collections::BTreeSet, time::SystemTime};

use color_eyre::Result;
use context_attribute::context;
use filtering::kalman_filter::KalmanFilter;
use framework::{HistoricInput, MainOutput, PerceptionInput};
use hungarian_algorithm::AssignmentProblem;
use itertools::{Either, Itertools};
use nalgebra::{matrix, vector, Isometry2, Matrix2, Matrix2x4, Matrix4, Matrix4x2, Point2};
use ndarray::Array2;
use ordered_float::NotNan;
use projection::Projection;
use serde::{Deserialize, Serialize};
use types::{
    ball::Ball,
    camera_matrix::{CameraMatrices, CameraMatrix},
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::BoundingBox,
    robot_filter::{Hypothesis, Measurement},
};

#[derive(Deserialize, Serialize)]
pub struct RobotFilter {
    hypotheses: Vec<Hypothesis>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    current_odometry_to_last_odometry:
        Input<Option<Isometry2<f32>>, "current_odometry_to_last_odometry?">,
    historic_current_odometry_to_last_odometry:
        HistoricInput<Option<Isometry2<f32>>, "current_odometry_to_last_odometry?">,
    historic_camera_matrices: HistoricInput<Option<CameraMatrices>, "camera_matrices?">,

    validity_threshold: Parameter<f32, "robot_filter.validity_threshold">,

    camera_matrices: RequiredInput<Option<CameraMatrices>, "camera_matrices?">,
    cycle_time: Input<CycleTime, "cycle_time">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    balls_bottom: PerceptionInput<Option<Vec<Ball>>, "VisionBottom", "balls?">,
    balls_top: PerceptionInput<Option<Vec<Ball>>, "VisionTop", "balls?">,
    robot_detections: PerceptionInput<Option<Vec<BoundingBox>>, "DetectionTop", "detections?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_position: MainOutput<Vec<Point2<f32>>>,
}

impl RobotFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            hypotheses: Vec::new(),
        })
    }

    fn persistent_robots_in_control_cycle<'a>(
        context: &'a CycleContext,
    ) -> Vec<(&'a SystemTime, Vec<&'a BoundingBox>)> {
        context
            .robot_detections
            .persistent
            .iter()
            .map(|(detection_time, robots)| {
                let robots = robots
                    .iter()
                    .filter_map(|robots| robots.as_ref())
                    .flat_map(|robots| robots.iter())
                    .collect();
                (detection_time, robots)
            })
            .collect()
    }

    fn advance_all_hypotheses(
        &mut self,
        measurements: Vec<(&SystemTime, Vec<&BoundingBox>)>,
        context: &CycleContext,
    ) {
        let param_process_noise = vector![0.1, 0.1, 0.5, 0.5];

        for (detection_time, robots) in measurements {
            let current_odometry_to_last_odometry = context
                .historic_current_odometry_to_last_odometry
                .get(detection_time)
                .expect("current_odometry_to_last_odometry should not be None");

            let last_camera_matrices = context
                .historic_camera_matrices
                .get(detection_time)
                .expect("camera_matrices should not be None");

            self.predict_hypotheses_with_odometry(
                current_odometry_to_last_odometry.inverse(),
                Matrix4::from_diagonal(&param_process_noise),
            );

            // match measured robots to hypotheses
            let measurements = Self::collect_measurements(robots, &last_camera_matrices.top);
            self.update_hypotheses_with_measurements(&measurements, *detection_time);
        }
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let persistent_updates = Self::persistent_robots_in_control_cycle(&context);
        self.advance_all_hypotheses(persistent_updates, &context);

        let robot_positions: Vec<Point2<f32>> = self
            .hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.validity > *context.validity_threshold)
            .map(|hypothesis| hypothesis.bounding_box.mean.xy().into())
            .collect();

        Ok(MainOutputs {
            robot_position: robot_positions.into(),
        })
    }

    fn predict_hypotheses_with_odometry(
        &mut self,
        last_odometry_to_current_odometry: Isometry2<f32>,
        process_noise: Matrix4<f32>,
    ) {
        for hypothesis in self.hypotheses.iter_mut() {
            let cycle_time = 0.012;
            let dt = cycle_time;
            // state is: (x, y, vx, vy)
            let constant_velocity_prediction = matrix![
                1.0, 0.0, dt, 0.0;
                0.0, 1.0, 0.0, dt;
                0.0, 0.0, 1.0, 0.0;
                0.0, 0.0, 0.0, 1.0;
            ];

            let rotation = last_odometry_to_current_odometry
                .rotation
                .to_rotation_matrix();

            let state_rotation = matrix![
                rotation[(0, 0)], rotation[(0, 1)], 0.0, 0.0;
                rotation[(1, 0)], rotation[(1, 1)], 0.0, 0.0;
                0.0, 0.0, rotation[(0, 0)], rotation[(0, 1)];
                0.0, 0.0, rotation[(1, 0)], rotation[(1, 1)];
            ];

            let state_prediction = constant_velocity_prediction * state_rotation;
            let control_input_model = Matrix4x2::identity();

            let odometry_translation = last_odometry_to_current_odometry.translation.vector;
            hypothesis.bounding_box.predict(
                state_prediction,
                control_input_model,
                odometry_translation,
                process_noise,
            );
        }
    }

    fn collect_measurements(
        detections: Vec<&BoundingBox>,
        camera_matrix: &CameraMatrix,
    ) -> Vec<Measurement> {
        detections
            .into_iter()
            .filter_map(|detection| {
                camera_matrix
                    .pixel_to_ground_with_z(detection.bottom_center(), 0.0)
                    .ok()
                    .map(|location| {
                        let projected_error = Matrix2::identity();
                        Measurement {
                            location,
                            score: detection.score,
                            projected_error,
                        }
                    })
            })
            .collect()
    }

    fn compute_distance_matrix(&self, measurements: &Vec<Measurement>) -> Array2<NotNan<f32>> {
        let observation_matrix = matrix![
            1.0, 0.0, 0.0, 0.0;
            0.0, 1.0, 0.0, 0.0;
        ];

        Array2::from_shape_fn(
            (measurements.len(), self.hypotheses.len()),
            |(projected_measurement, hypothesis)| {
                let observation = self.hypotheses[hypothesis].bounding_box;
                let measurement = measurements[projected_measurement];

                // Instead could also do: Matrix2::from_diagonal(&hypothesis.bounding_box.mean.xy())
                let residual_distance =
                    measurement.location.coords - observation_matrix * observation.mean;

                // Same here
                let residual_covariance =
                    observation_matrix * observation.covariance * observation_matrix.transpose()
                        + measurement.projected_error;

                let normalized_mahalanobis_distance = (residual_distance.transpose()
                    * residual_covariance.lu().solve(&residual_distance).unwrap())
                .x + residual_covariance.determinant().ln();

                NotNan::new(normalized_mahalanobis_distance).unwrap()
            },
        )
    }

    fn update_hypotheses_with_measurements(
        &mut self,
        measurements: &Vec<Measurement>,
        detection_time: SystemTime,
    ) {
        let distance_metrics = self.compute_distance_matrix(measurements);

        let assignment = AssignmentProblem::from_costs(distance_metrics).solve();

        let (associated_hypotheses, remaining_hypotheses): (Vec<_>, Vec<_>) = self
            .hypotheses
            .into_iter()
            .enumerate()
            .partition_map(|(index, hypothesis)| match assignment[index] {
                Some(measurement_index) => {
                    Either::Left((hypothesis, measurements[measurement_index]))
                }
                None => Either::Right(hypothesis),
            });
        self.hypotheses.clear();

        for (hypothesis, measurement) in associated_hypotheses {
            hypothesis.update(Matrix2x4::identity(), measurement);
            self.hypotheses.push(hypothesis);
        }

        for hypothesis in remaining_hypotheses {
            if detection_time
                .duration_since(hypothesis.last_update)
                .expect("time ran backwards")
                .as_secs_f32()
                < 2.0
            {
                self.hypotheses.push(hypothesis);
            }
        }

        let mut remaining_detections: BTreeSet<usize> =
            (0..measurements.len()).into_iter().collect();
        for task in assignment {
            if let Some(task) = task {
                remaining_detections.remove(&task);
            }
        }

        for detection in remaining_detections.iter().filter_map(|&index| {
            if measurements[index].score > 0.5 {
                Some(measurements[index])
            } else {
                None
            }
        }) {
            self.spawn_hypothesis(&detection, detection_time);
        }

        for hypothesis in self.hypotheses.iter_mut() {
            hypothesis.last_update = detection_time;
        }
    }

    fn spawn_hypothesis(&mut self, measurement: &Measurement, detection_time: SystemTime) {
        let initial_state = vector![measurement.location.x, measurement.location.y, 0.0, 0.0];
        let new_hypothesis = Hypothesis {
            bounding_box: MultivariateNormalDistribution {
                mean: initial_state,
                covariance: Matrix4::from_diagonal(&configuration.initial_covariance),
            },
            validity: 1.0,
            last_update: detection_time,
        };
        self.hypotheses.push(new_hypothesis);
    }
}
