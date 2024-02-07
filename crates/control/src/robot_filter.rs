use std::{ops::Bound, time::SystemTime};

use color_eyre::Result;
use context_attribute::context;
use filtering::kalman_filter::KalmanFilter;
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use geometry::circle::Circle;
use nalgebra::{
    linalg::LU, matrix, vector, Affine2, ArrayStorage, Isometry2, Matrix, Matrix2, Matrix2x4,
    Matrix4, Matrix4x2, Perspective3, Point2, Similarity2, U8,
};
use projection::Projection;
use serde::{Deserialize, Serialize};
use types::{
    ball::Ball,
    camera_matrix::{CameraMatrices, CameraMatrix},
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::BoundingBox,
    robot_filter::Hypothesis,
};

use crate::ground_contact_detector;

type Matrix8<T> = Matrix<T, U8, U8, ArrayStorage<T, 8, 8>>;

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
    pub robot_position: MainOutput<Vec<BoundingBox>>,
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
                .current_odometry_to_last_odometry
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
            self.update_hypotheses_with_measurements(robots, *detection_time);
        }

        self.remove_hypotheses(
            context.cycle_time.start_time,
            context.ball_filter_configuration,
            context.field_dimensions,
        );
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let persistent_updates = Self::persistent_robots_in_control_cycle(&context);
        self.advance_all_hypotheses(persistent_updates, &context);

        context
            .ball_filter_hypotheses
            .fill_if_subscribed(|| self.hypotheses.clone());
        let ball_radius = context.field_dimensions.ball_radius;

        let ball_positions = self
            .hypotheses
            .iter()
            .map(|hypothesis| hypothesis.selected_ball_position(context.ball_filter_configuration))
            .collect::<Vec<_>>();

        let robot_positions = self
            .hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.validity > *context.validity_threshold)
            .collect();

        Ok(MainOutputs {
            robot_position: robot_positions.into(),
        })
    }

    fn decay_hypotheses(
        &mut self,
        camera_matrices: Option<&CameraMatrices>,
        projected_limbs: Option<&ProjectedLimbs>,
        ball_radius: f32,
        configuration: &BallFilterParameters,
    ) {
        for hypothesis in self.hypotheses.iter_mut() {
            let ball_in_view = match (camera_matrices.as_ref(), projected_limbs.as_ref()) {
                (Some(camera_matrices), Some(projected_limbs)) => {
                    is_visible_to_camera(
                        hypothesis,
                        &camera_matrices.bottom,
                        ball_radius,
                        &projected_limbs.limbs,
                        configuration,
                    ) || is_visible_to_camera(
                        hypothesis,
                        &camera_matrices.top,
                        ball_radius,
                        &[],
                        configuration,
                    )
                }
                _ => false,
            };

            let decay_factor = if ball_in_view {
                configuration.visible_validity_exponential_decay_factor
            } else {
                configuration.hidden_validity_exponential_decay_factor
            };
            hypothesis.validity *= decay_factor;
        }
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

    fn update_hypotheses_with_measurements(
        &mut self,
        detected_robots: &Vec<BoundingBox>,
        detection_time: SystemTime,
        camera_matrix: &CameraMatrix,
    ) {
        let projected_robot_positions: Vec<_> = detected_robots.into_iter().filter_map(|robot| {
            camera_matrix.pixel_to_ground(robot.bottom_center).ok()
        }).collect();

        for hypothesis in self.hypotheses {
            let observation = hypothesis.bounding_box;
            for projected_measurement in self.projected_robot_positions {

            }
        }
    }

    fn spawn_hypothesis(
        &mut self,
        detected_position: Point2<f32>,
        detection_time: SystemTime,
        configuration: &BallFilterParameters,
    ) {
        let initial_state = vector![
            detected_position.coords.x,
            detected_position.coords.y,
            0.0,
            0.0
        ];
        let new_hypothesis = Hypothesis {
            moving_state: MultivariateNormalDistribution {
                mean: initial_state,
                covariance: Matrix4::from_diagonal(&configuration.initial_covariance),
            },
            resting_state: MultivariateNormalDistribution {
                mean: initial_state,
                covariance: Matrix4::from_diagonal(&configuration.initial_covariance),
            },
            validity: 1.0,
            last_update: detection_time,
        };
        self.hypotheses.push(new_hypothesis);
    }

    fn remove_hypotheses(
        &mut self,
        now: SystemTime,
        configuration: &BallFilterParameters,
        field_dimensions: &FieldDimensions,
    ) {
        self.hypotheses.retain(|hypothesis| {
            let selected_position = hypothesis.selected_ball_position(configuration).position;
            let is_inside_field = {
                selected_position.coords.x.abs()
                    < field_dimensions.length / 2.0 + field_dimensions.border_strip_width
                    && selected_position.y.abs()
                        < field_dimensions.width / 2.0 + field_dimensions.border_strip_width
            };
            now.duration_since(hypothesis.last_update)
                .expect("Time has run backwards")
                < configuration.hypothesis_timeout
                && hypothesis.validity > configuration.validity_discard_threshold
                && is_inside_field
        });
        let mut deduplicated_hypotheses = Vec::<Hypothesis>::new();
        for hypothesis in self.hypotheses.drain(..) {
            let hypothesis_in_merge_distance =
                deduplicated_hypotheses
                    .iter_mut()
                    .find(|existing_hypothesis| {
                        (existing_hypothesis
                            .selected_ball_position(configuration)
                            .position
                            .coords
                            - hypothesis
                                .selected_ball_position(configuration)
                                .position
                                .coords)
                            .norm()
                            < configuration.hypothesis_merge_distance
                    });
            match hypothesis_in_merge_distance {
                Some(existing_hypothesis) => {
                    let update_state = hypothesis.selected_state(configuration);
                    existing_hypothesis.moving_state.update(
                        Matrix4::identity(),
                        update_state.mean,
                        update_state.covariance,
                    );
                    
                    existing_hypothesis.resting_state.update(
                        Matrix4::identity(),
                        update_state.mean,
                        update_state.covariance,
                    );
                }
                None => deduplicated_hypotheses.push(hypothesis),
            }
        }
        self.hypotheses = deduplicated_hypotheses;
    }
}

fn project_to_image(
    ball_position: &[BallPosition],
    camera_matrix: &CameraMatrix,
    ball_radius: f32,
) -> Vec<Circle> {
    ball_position
        .iter()
        .filter_map(|ball_position| {
            let position_in_image = camera_matrix
                .ground_with_z_to_pixel(ball_position.position, ball_radius)
                .ok()?;
            let radius = camera_matrix
                .get_pixel_radius(ball_radius, position_in_image, vector![640, 480])
                .ok()?;
            Some(Circle {
                center: position_in_image,
                radius,
            })
        })
        .collect()
}

fn is_visible_to_camera(
    hypothesis: &Hypothesis,
    camera_matrix: &CameraMatrix,
    ball_radius: f32,
    projected_limbs: &[Limb],
    configuration: &BallFilterParameters,
) -> bool {
    let position_on_ground = hypothesis.selected_ball_position(configuration).position;
    let position_in_image =
        match camera_matrix.ground_with_z_to_pixel(position_on_ground, ball_radius) {
            Ok(position_in_image) => position_in_image,
            Err(_) => return false,
        };
    (0.0..640.0).contains(&position_in_image.x)
        && (0.0..480.0).contains(&position_in_image.y)
        && is_above_limbs(position_in_image, projected_limbs)
}
