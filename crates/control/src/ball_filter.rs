use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use filtering::KalmanFilter;
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use nalgebra::{
    matrix, vector, Isometry2, Matrix2, Matrix2x4, Matrix4, Matrix4x2, Point2, Vector2, Vector4,
};
use types::{
    is_above_limbs, Ball, BallFilterHypothesis, BallPosition, CameraMatrices, CameraMatrix, Circle,
    CycleTime, FieldDimensions, Limb, ProjectedLimbs, SensorData,
};

pub struct BallFilter {
    hypotheses: Vec<BallFilterHypothesis>,
}

#[context]
pub struct CreationContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub hidden_validity_exponential_decay_factor:
        Parameter<f32, "control.ball_filter.hidden_validity_exponential_decay_factor">,
    pub hypothesis_merge_distance: Parameter<f32, "control.ball_filter.hypothesis_merge_distance">,
    pub hypothesis_timeout: Parameter<Duration, "control.ball_filter.hypothesis_timeout">,
    pub initial_covariance: Parameter<Vector4<f32>, "control.ball_filter.initial_covariance">,
    pub measurement_matching_distance:
        Parameter<f32, "control.ball_filter.measurement_matching_distance">,
    pub measurement_noise: Parameter<Vector2<f32>, "control.ball_filter.measurement_noise">,
    pub process_noise: Parameter<Vector4<f32>, "control.ball_filter.process_noise">,
    pub validity_discard_threshold:
        Parameter<f32, "control.ball_filter.validity_discard_threshold">,
    pub visible_validity_exponential_decay_factor:
        Parameter<f32, "control.ball_filter.visible_validity_exponential_decay_factor">,
}

#[context]
pub struct CycleContext {
    pub ball_filter_hypotheses:
        AdditionalOutput<Vec<BallFilterHypothesis>, "ball_filter_hypotheses">,
    pub filtered_balls_in_image_bottom:
        AdditionalOutput<Vec<Circle>, "filtered_balls_in_image_bottom">,
    pub filtered_balls_in_image_top: AdditionalOutput<Vec<Circle>, "filtered_balls_in_image_top">,

    pub current_odometry_to_last_odometry:
        HistoricInput<Option<Isometry2<f32>>, "current_odometry_to_last_odometry?">,
    pub historic_camera_matrices: HistoricInput<Option<CameraMatrices>, "camera_matrices?">,
    pub projected_limbs: HistoricInput<Option<ProjectedLimbs>, "projected_limbs?">,

    pub camera_matrices: RequiredInput<Option<CameraMatrices>, "camera_matrices?">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,

    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub hidden_validity_exponential_decay_factor:
        Parameter<f32, "control.ball_filter.hidden_validity_exponential_decay_factor">,
    pub hypothesis_merge_distance: Parameter<f32, "control.ball_filter.hypothesis_merge_distance">,
    pub hypothesis_timeout: Parameter<Duration, "control.ball_filter.hypothesis_timeout">,
    pub initial_covariance: Parameter<Vector4<f32>, "control.ball_filter.initial_covariance">,
    pub measurement_matching_distance:
        Parameter<f32, "control.ball_filter.measurement_matching_distance">,
    pub measurement_noise: Parameter<Vector2<f32>, "control.ball_filter.measurement_noise">,
    pub process_noise: Parameter<Vector4<f32>, "control.ball_filter.process_noise">,
    pub validity_discard_threshold:
        Parameter<f32, "control.ball_filter.validity_discard_threshold">,
    pub visible_validity_exponential_decay_factor:
        Parameter<f32, "control.ball_filter.visible_validity_exponential_decay_factor">,

    pub balls_bottom: PerceptionInput<Option<Vec<Ball>>, "VisionBottom", "balls?">,
    pub balls_top: PerceptionInput<Option<Vec<Ball>>, "VisionTop", "balls?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ball_position: MainOutput<Option<BallPosition>>,
}

impl BallFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            hypotheses: Vec::new(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let measured_balls = context
            .balls_top
            .persistent
            .iter()
            .zip(context.balls_bottom.persistent.values());
        for ((detection_time, balls_top), balls_bottom) in measured_balls {
            let current_odometry_to_last_odometry = context
                .current_odometry_to_last_odometry
                .get(detection_time)
                .expect("current_odometry_to_last_odometry should not be None");
            let measured_balls_in_control_cycle = balls_top
                .iter()
                .chain(balls_bottom.iter())
                .filter_map(|data| data.as_ref());
            self.predict_hypotheses_with_odometry(
                current_odometry_to_last_odometry.inverse(),
                Matrix4::from_diagonal(context.process_noise),
            );

            let camera_matrices = context.historic_camera_matrices.get(detection_time);
            let projected_limbs_bottom = context.projected_limbs.get(detection_time);
            self.decay_hypotheses(
                camera_matrices,
                projected_limbs_bottom,
                context.field_dimensions.ball_radius,
                *context.visible_validity_exponential_decay_factor,
                *context.hidden_validity_exponential_decay_factor,
            );

            for balls in measured_balls_in_control_cycle {
                for ball in *balls {
                    self.update_hypotheses_with_measurement(
                        ball.position,
                        *detection_time,
                        *context.measurement_matching_distance,
                        Matrix2::from_diagonal(context.measurement_noise),
                        Matrix4::from_diagonal(context.initial_covariance),
                    );
                }
            }
        }

        self.remove_hypotheses(
            context.cycle_time.start_time,
            *context.hypothesis_merge_distance,
            *context.hypothesis_timeout,
            *context.validity_discard_threshold,
            context.field_dimensions,
        );

        let best_hypothesis = self.find_best_hypothesis();
        let ball_position = best_hypothesis.map(|hypothesis| BallPosition {
            position: Point2::from(hypothesis.filter.state().xy()),
            last_seen: hypothesis.last_update,
        });
        context
            .ball_filter_hypotheses
            .fill_on_subscription(|| self.hypotheses.clone());
        let ball_radius = context.field_dimensions.ball_radius;
        context
            .filtered_balls_in_image_top
            .fill_on_subscription(|| {
                self.hypotheses
                    .iter()
                    .filter_map(|hypothesis| {
                        project_to_image(hypothesis, &context.camera_matrices.top, ball_radius)
                    })
                    .collect()
            });
        context
            .filtered_balls_in_image_bottom
            .fill_on_subscription(|| {
                self.hypotheses
                    .iter()
                    .filter_map(|hypothesis| {
                        project_to_image(hypothesis, &context.camera_matrices.bottom, ball_radius)
                    })
                    .collect()
            });
        Ok(MainOutputs {
            ball_position: ball_position.into(),
        })
    }

    fn decay_hypotheses(
        &mut self,
        camera_matrices: Option<&CameraMatrices>,
        projected_limbs: Option<&ProjectedLimbs>,
        ball_radius: f32,
        visible_validity_exponential_decay_factor: f32,
        hidden_validity_exponential_decay_factor: f32,
    ) {
        for hypothesis in self.hypotheses.iter_mut() {
            let ball_in_view = match (camera_matrices.as_ref(), projected_limbs.as_ref()) {
                (Some(camera_matrices), Some(projected_limbs)) => is_visible_to_camera(
                    hypothesis,
                    &camera_matrices.bottom,
                    ball_radius,
                    &projected_limbs.bottom,
                ),
                _ => false,
            };

            let decay_factor = if ball_in_view {
                visible_validity_exponential_decay_factor
            } else {
                hidden_validity_exponential_decay_factor
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
            let constant_velocity_prediction = matrix![
                1.0, 0.0, cycle_time, 0.0;
                0.0, 1.0, 0.0, cycle_time;
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
            hypothesis.filter.predict(
                state_prediction,
                control_input_model,
                odometry_translation,
                process_noise,
            )
        }
    }

    fn update_hypotheses_with_measurement(
        &mut self,
        detected_position: Point2<f32>,
        detection_time: SystemTime,
        matching_distance: f32,
        measurement_noise: Matrix2<f32>,
        initial_covariance: Matrix4<f32>,
    ) {
        let mut matching_hypotheses = self
            .hypotheses
            .iter_mut()
            .filter(|hypothesis| {
                (hypothesis.filter.state().xy() - detected_position.coords).norm()
                    < matching_distance
            })
            .peekable();
        if matching_hypotheses.peek().is_none() {
            self.spawn_hypothesis(detected_position, detection_time, initial_covariance);
            return;
        }
        matching_hypotheses.for_each(|hypothesis| {
            hypothesis.filter.update(
                Matrix2x4::identity(),
                detected_position.coords,
                measurement_noise * detected_position.coords.norm_squared(),
            );
            hypothesis.validity += 1.0;
            hypothesis.last_update = detection_time;
        });
    }

    fn find_best_hypothesis(&self) -> Option<&BallFilterHypothesis> {
        self.hypotheses
            .iter()
            .max_by(|a, b| a.validity.total_cmp(&b.validity))
    }

    fn spawn_hypothesis(
        &mut self,
        detected_position: Point2<f32>,
        detection_time: SystemTime,
        initial_covariance: Matrix4<f32>,
    ) {
        let initial_state = vector![
            detected_position.coords.x,
            detected_position.coords.y,
            0.0,
            0.0
        ];
        let new_hypothesis = BallFilterHypothesis {
            filter: KalmanFilter::new(initial_state, initial_covariance),
            validity: 1.0,
            last_update: detection_time,
        };
        self.hypotheses.push(new_hypothesis);
    }

    fn remove_hypotheses(
        &mut self,
        now: SystemTime,
        merge_distance: f32,
        hypothesis_timeout: Duration,
        validity_discard_threshold: f32,
        field_dimensions: &FieldDimensions,
    ) {
        self.hypotheses.retain(|hypothesis| {
            let position = hypothesis.filter.state().xy();
            let is_inside_field = position.x.abs()
                < field_dimensions.length / 2.0 + field_dimensions.border_strip_width
                && position.y.abs()
                    < field_dimensions.width / 2.0 + field_dimensions.border_strip_width;
            now.duration_since(hypothesis.last_update)
                .expect("Time has run backwards")
                < hypothesis_timeout
                && hypothesis.validity > validity_discard_threshold
                && is_inside_field
        });
        let mut deduplicated_hypotheses = Vec::<BallFilterHypothesis>::new();
        for hypothesis in self.hypotheses.drain(..) {
            let hypothesis_in_merge_distance =
                deduplicated_hypotheses
                    .iter_mut()
                    .find(|existing_hypothesis| {
                        (existing_hypothesis.filter.state().xy() - hypothesis.filter.state().xy())
                            .norm()
                            < merge_distance
                    });
            match hypothesis_in_merge_distance {
                Some(existing_hypothesis) => {
                    existing_hypothesis.filter.update(
                        Matrix4::identity(),
                        hypothesis.filter.state(),
                        hypothesis.filter.covariance(),
                    );
                }
                None => deduplicated_hypotheses.push(hypothesis),
            }
        }
        self.hypotheses = deduplicated_hypotheses;
    }
}

fn project_to_image(
    hypothesis: &BallFilterHypothesis,
    camera_matrix: &CameraMatrix,
    ball_radius: f32,
) -> Option<Circle> {
    let pixel_position = camera_matrix
        .ground_with_z_to_pixel(&Point2::from(hypothesis.filter.state().xy()), ball_radius)
        .ok()?;
    let radius = camera_matrix
        .get_pixel_radius(ball_radius, &pixel_position, &vector![640, 480])
        .ok()?;
    Some(Circle {
        center: pixel_position,
        radius,
    })
}

fn is_visible_to_camera(
    hypothesis: &BallFilterHypothesis,
    camera_matrix: &CameraMatrix,
    ball_radius: f32,
    projected_limbs_bottom: &[Limb],
) -> bool {
    let position_in_image = match camera_matrix
        .ground_with_z_to_pixel(&Point2::from(hypothesis.filter.state().xy()), ball_radius)
    {
        Ok(position_in_image) => position_in_image,
        Err(_) => return false,
    };
    (0.0..640.0).contains(&position_in_image.x)
        && (0.0..480.0).contains(&position_in_image.y)
        && is_above_limbs(position_in_image, projected_limbs_bottom)
}
