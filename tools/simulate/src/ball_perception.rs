//! Synthetic detector output at the production vision/filter boundary.
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use color_eyre::Result;
use coordinate_systems::{Field, Ground, Pixel};
use geometry::rectangle::Rectangle;
use linear_algebra::{Isometry2, Point2, Point3, point};
use projection::{Projection, camera_matrix::CameraMatrix};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, StandardNormal};
use ros_z::{prelude::*, time::Time};
use ros_z_streams::{AnnouncingPublisher, CreateAnnouncingPublisher};
use serde::{Deserialize, Serialize};
use tokio::{runtime::Handle, task::JoinHandle};
use types::{
    ball_position::BallPosition,
    bounding_box::BoundingBox,
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(default, deny_unknown_fields)]
pub struct Parameters {
    /// Logical seconds between camera frames, independent of rendering speed.
    pub frame_period: f64,
    /// Independent standard deviation of bounding-box center x/y, in pixels.
    pub center_noise_pixels: f32,
    /// Fixed detector-center bias per recording, in pixels.
    pub center_bias_pixels: [f32; 2],
    /// Independent probability of suppressing true detections on a frame.
    pub dropout_probability: f32,
    /// Probability of starting a correlated run of missed true detections.
    pub dropout_burst_probability: f32,
    pub dropout_burst_frames: u32,
    /// Frames for which an injected false detection persists near its initial pixel.
    pub false_positive_burst_frames: u32,
    /// Probability of one additional uniformly distributed image detection per frame.
    pub false_positive_probability: f32,
    pub detection_confidence: f32,
    /// Pixel radius of the additional false detection.
    pub false_positive_radius: f32,
    pub seed: u64,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            frame_period: 0.04,
            center_noise_pixels: 2.0,
            center_bias_pixels: [0.0; 2],
            dropout_probability: 0.0,
            dropout_burst_probability: 0.0,
            dropout_burst_frames: 1,
            false_positive_burst_frames: 1,
            false_positive_probability: 0.04,
            detection_confidence: 0.9,
            false_positive_radius: 8.0,
            seed: 42,
        }
    }
}

impl Parameters {
    pub fn validate(&self) -> Result<(), String> {
        if !self.frame_period.is_finite() || self.frame_period < 0.002 {
            return Err(
                "ball_perception.frame_period must be finite and at least 0.002 seconds".into(),
            );
        }
        if !self.center_noise_pixels.is_finite() || self.center_noise_pixels < 0.0 {
            return Err(
                "ball_perception.center_noise_pixels must be finite and nonnegative".into(),
            );
        }
        if !self.center_bias_pixels.iter().all(|v| v.is_finite())
            || self.dropout_burst_frames == 0
            || self.false_positive_burst_frames == 0
        {
            return Err("detector biases must be finite and burst lengths must be positive".into());
        }
        for (name, value) in [
            ("dropout_probability", self.dropout_probability),
            ("dropout_burst_probability", self.dropout_burst_probability),
            (
                "false_positive_probability",
                self.false_positive_probability,
            ),
            ("detection_confidence", self.detection_confidence),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(format!(
                    "ball_perception.{name} must be between zero and one"
                ));
            }
        }
        if !self.false_positive_radius.is_finite() || self.false_positive_radius <= 0.0 {
            return Err("ball_perception.false_positive_radius must be finite and positive".into());
        }
        Ok(())
    }
}

pub struct Detector {
    rng: ChaCha8Rng,
    seed: u64,
    dropout_remaining: u32,
    false_remaining: u32,
    false_center: Point2<Pixel>,
}

/// Upright physical occluder in the same ground frame as the camera and balls.
pub struct Occluder {
    pub center: Point3<Ground>,
    pub radius: f32,
    pub height: f32,
}

pub(crate) fn occludes(camera: &CameraMatrix, ball: Point3<Ground>, obstacle: &Occluder) -> bool {
    let origin = camera.ground_to_camera.inverse().inner.translation.vector;
    let delta = ball.inner.coords - origin;
    let relative = origin - obstacle.center.inner.coords;
    let a = delta.xy().norm_squared();
    let c = relative.xy().norm_squared() - obstacle.radius.powi(2);
    let (mut enter, mut exit) = (0.0_f32, 1.0_f32);
    if a < 1e-12 {
        if c > 0.0 {
            return false;
        }
    } else {
        let b = relative.xy().dot(&delta.xy());
        let discriminant = b * b - a * c;
        if discriminant < 0.0 {
            return false;
        }
        let root = discriminant.sqrt();
        enter = enter.max((-b - root) / a);
        exit = exit.min((-b + root) / a);
    }
    if delta.z.abs() < 1e-6 {
        if relative.z.abs() > obstacle.height / 2.0 {
            return false;
        }
    } else {
        let low = (-obstacle.height / 2.0 - relative.z) / delta.z;
        let high = (obstacle.height / 2.0 - relative.z) / delta.z;
        enter = enter.max(low.min(high));
        exit = exit.min(low.max(high));
    }
    enter < exit && exit > 0.0 && enter < 1.0
}

impl Detector {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            seed,
            dropout_remaining: 0,
            false_remaining: 0,
            false_center: Point2::origin(),
        }
    }

    pub fn detect(
        &mut self,
        camera: &CameraMatrix,
        balls: &[Point3<Ground>],
        radius: f32,
        parameters: &Parameters,
    ) -> Vec<Object<RobocupObjectLabel>> {
        if self.seed != parameters.seed {
            *self = Self::new(parameters.seed);
        }
        let mut detections = Vec::new();
        if self.dropout_remaining == 0
            && parameters.dropout_burst_probability > 0.0
            && self.rng.random::<f32>() < parameters.dropout_burst_probability
        {
            self.dropout_remaining = parameters.dropout_burst_frames;
        }
        let drop_frame = self.dropout_remaining > 0
            || (parameters.dropout_probability > 0.0
                && self.rng.random::<f32>() < parameters.dropout_probability);
        self.dropout_remaining = self.dropout_remaining.saturating_sub(1);
        for ball in balls.iter().filter(|_| !drop_frame) {
            let Ok(center) = camera.ground_with_z_to_pixel(ball.xy(), ball.z()) else {
                continue;
            };
            if !in_image(camera, center) {
                continue;
            }
            let in_camera = camera.ground_to_camera * *ball;
            // Pinhole approximation for a sphere, using its actual depth (including flight).
            let pixel_radius =
                radius * camera.intrinsics.focals.x.min(camera.intrinsics.focals.y) / in_camera.z();
            if !pixel_radius.is_finite() || pixel_radius <= 0.0 {
                continue;
            }
            let dx: f32 = StandardNormal.sample(&mut self.rng);
            let dy: f32 = StandardNormal.sample(&mut self.rng);
            let noisy = point![
                center.x() + dx * parameters.center_noise_pixels + parameters.center_bias_pixels[0],
                center.y() + dy * parameters.center_noise_pixels + parameters.center_bias_pixels[1]
            ];
            if in_image(camera, noisy) {
                detections.push(detection(
                    noisy,
                    pixel_radius,
                    parameters.detection_confidence,
                ));
            }
        }
        if self.false_remaining == 0
            && self.rng.random::<f32>() < parameters.false_positive_probability
        {
            self.false_center = point![
                self.rng.random::<f32>() * camera.image_size.x(),
                self.rng.random::<f32>() * camera.image_size.y()
            ];
            self.false_remaining = parameters.false_positive_burst_frames;
        }
        if self.false_remaining > 0 {
            detections.push(detection(
                self.false_center,
                parameters.false_positive_radius,
                parameters.detection_confidence,
            ));
            self.false_remaining -= 1;
        }
        detections
    }
}

pub(crate) fn in_image(camera: &CameraMatrix, point: Point2<Pixel>) -> bool {
    (0.0..camera.image_size.x()).contains(&point.x())
        && (0.0..camera.image_size.y()).contains(&point.y())
}

fn detection(center: Point2<Pixel>, radius: f32, confidence: f32) -> Object<RobocupObjectLabel> {
    Object {
        label: RobocupObjectLabel::Ball,
        bounding_box: BoundingBox {
            area: Rectangle {
                min: point![center.x() - radius, center.y() - radius],
                max: point![center.x() + radius, center.y() + radius],
            },
            confidence,
        },
    }
}

/// Metrics are conditional on an estimate and real ball existing; misses and false tracks
/// are counted separately because conditional position error alone rewards suppression.
#[derive(Debug, Default, Clone, Serialize, Deserialize, Message)]
pub struct Metrics {
    pub time: Time,
    pub matched_samples: u64,
    pub missing_estimates: u64,
    pub estimates_without_ball: u64,
    pub empty_samples: u64,
    pub unmatched_timestamps: u64,
    pub position_error_metres: Option<f64>,
    pub position_rmse_metres: Option<f64>,
    pub field_matched_samples: u64,
    pub missing_field_transforms: u64,
    pub field_transform_age_seconds: Option<f64>,
    pub field_position_error_metres: Option<f64>,
    pub field_position_rmse_metres: Option<f64>,
}

impl Metrics {
    fn observe(
        &mut self,
        time: Time,
        truth: &[Point3<Ground>],
        estimate: Option<BallPosition<Ground>>,
    ) {
        self.time = time;
        self.position_error_metres = None;
        match (truth.is_empty(), estimate) {
            (true, Some(_)) => self.estimates_without_ball += 1,
            (true, None) => self.empty_samples += 1,
            (false, None) => self.missing_estimates += 1,
            (false, Some(estimate)) => {
                let error = truth
                    .iter()
                    .map(|ball| (ball.xy() - estimate.position).norm() as f64)
                    .min_by(f64::total_cmp)
                    .expect("truth is nonempty");
                self.matched_samples += 1;
                let mean_square = self.position_rmse_metres.unwrap_or(0.0).powi(2);
                self.position_error_metres = Some(error);
                self.position_rmse_metres = Some(
                    (mean_square + (error * error - mean_square) / self.matched_samples as f64)
                        .sqrt(),
                );
            }
        }
    }

    fn observe_field(
        &mut self,
        truth: &TruthFrame,
        estimate: Option<BallPosition<Ground>>,
        pose: Option<(Time, Isometry2<Ground, Field>)>,
    ) {
        self.field_position_error_metres = None;
        self.field_transform_age_seconds = None;
        let Some(estimate) = estimate.filter(|_| !truth.balls.is_empty()) else {
            return;
        };
        let Some((stamp, pose)) = pose
            .filter(|(stamp, _)| self.time.duration_since(*stamp) <= Duration::from_millis(100))
        else {
            self.missing_field_transforms += 1;
            return;
        };
        self.field_transform_age_seconds = Some(self.time.duration_since(stamp).as_secs_f64());
        let position = pose * estimate.position;
        let error = truth
            .balls
            .iter()
            .map(|ball| (truth.ground_to_field * ball.xy() - position).norm() as f64)
            .min_by(f64::total_cmp)
            .expect("truth is nonempty");
        self.field_matched_samples += 1;
        let mean_square = self.field_position_rmse_metres.unwrap_or(0.0).powi(2);
        self.field_position_error_metres = Some(error);
        self.field_position_rmse_metres = Some(
            (mean_square + (error * error - mean_square) / self.field_matched_samples as f64)
                .sqrt(),
        );
    }
}

#[derive(Clone)]
struct TruthFrame {
    balls: Vec<Point3<Ground>>,
    ground_to_field: Isometry2<Ground, Field>,
}

type TruthHistory = Arc<Mutex<BTreeMap<Time, TruthFrame>>>;

pub struct PerceptionIo {
    runtime: Handle,
    detector: Detector,
    detections: AnnouncingPublisher<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>,
    truth: Publisher<TimeWrapper<Vec<Point3<Ground>>>>,
    field_truth: Publisher<TimeWrapper<Vec<Point3<Field>>>>,
    history: TruthHistory,
    metrics_task: JoinHandle<()>,
    last_frame: Option<Time>,
}

impl PerceptionIo {
    pub async fn new(node: &Node, runtime: &Handle) -> Result<Self> {
        let detections = node.announcing_publisher("detected_objects").await?;
        let truth = node
            .publisher("simulation/ball_ground_truth")
            .build()
            .await?;
        let field_truth = node
            .publisher("simulation/ball_ground_truth_field")
            .build()
            .await?;
        let estimates = node
            .subscriber::<Option<BallPosition<Ground>>>("ball_filter/ball_position")
            .build()
            .await?;
        let metrics_pub = node
            .publisher::<Metrics>("simulation/ball_filter_metrics")
            .build()
            .await?;
        let poses = node
            .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
            .build()
            .await?;
        let history = TruthHistory::default();
        let metric_history = history.clone();
        let metrics_task = runtime.spawn(async move {
            let mut metrics = Metrics::default();
            let mut last_time = None;
            let mut field_poses = BTreeMap::new();
            loop {
                let sample = tokio::select! {
                    pose = poses.recv_with_metadata() => {
                        let Ok(pose) = pose else { break; };
                        field_poses.insert(pose.source_time, pose.message);
                        while field_poses.len() > 10_000 { field_poses.pop_first(); }
                        continue;
                    }
                    sample = estimates.recv_with_metadata() => {
                        let Ok(sample) = sample else { break; };
                        sample
                    }
                };
                if last_time.is_some_and(|last| sample.source_time <= last) {
                    continue;
                }
                last_time = Some(sample.source_time);
                let truth = metric_history
                    .lock()
                    .expect("truth history lock poisoned")
                    .get(&sample.source_time)
                    .cloned();
                if let Some(truth) = truth {
                    metrics.observe(sample.source_time, &truth.balls, sample.message);
                    metrics.observe_field(
                        &truth,
                        sample.message,
                        field_poses
                            .range(..=sample.source_time)
                            .next_back()
                            .map(|(time, pose)| (*time, *pose)),
                    );
                } else {
                    metrics.time = sample.source_time;
                    metrics.position_error_metres = None;
                    metrics.field_position_error_metres = None;
                    metrics.field_transform_age_seconds = None;
                    metrics.unmatched_timestamps += 1;
                }
                if let Err(error) = metrics_pub
                    .publish_with_source_time(&metrics, sample.source_time)
                    .await
                {
                    bevy::log::warn!("cannot publish ball filter metrics: {error}");
                    break;
                }
            }
        });
        Ok(Self {
            runtime: runtime.clone(),
            detector: Detector::new(42),
            detections,
            truth,
            field_truth,
            history,
            metrics_task,
            last_frame: None,
        })
    }

    pub fn publish(
        &mut self,
        time: Time,
        ground_to_field: Isometry2<Ground, Field>,
        camera: &CameraMatrix,
        balls: Vec<Point3<Ground>>,
        radius: f32,
        parameters: &Parameters,
    ) -> Result<()> {
        self.publish_with_occluders(
            time,
            ground_to_field,
            camera,
            balls,
            radius,
            parameters,
            &[],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn publish_with_occluders(
        &mut self,
        time: Time,
        ground_to_field: Isometry2<Ground, Field>,
        camera: &CameraMatrix,
        balls: Vec<Point3<Ground>>,
        radius: f32,
        parameters: &Parameters,
        obstacles: &[Occluder],
    ) -> Result<()> {
        // Keep exact ground truth for delayed filter outputs, before announcing either input.
        {
            let mut history = self.history.lock().expect("truth history lock poisoned");
            history.insert(
                time,
                TruthFrame {
                    balls: balls.clone(),
                    ground_to_field,
                },
            );
            while history.len() > 10_000 {
                history.pop_first();
            }
        }
        let frame_due = self.last_frame.is_none_or(|last| {
            time.duration_since(last).as_secs_f64() + 1e-9 >= parameters.frame_period
        });
        self.runtime.block_on(async {
            // Record reference positions at physics cadence, matching odometry timestamps.
            // An absent message is unknown; an empty vector explicitly means no ball.
            self.truth
                .publish_with_source_time(
                    &TimeWrapper {
                        time,
                        inner: balls.clone(),
                    },
                    time,
                )
                .await?;
            self.field_truth
                .publish_with_source_time(
                    &TimeWrapper {
                        time,
                        inner: balls
                            .iter()
                            .map(|ball| {
                                let field = ground_to_field * ball.xy();
                                point![field.x(), field.y(), ball.z()]
                            })
                            .collect(),
                    },
                    time,
                )
                .await?;
            if frame_due {
                let visible: Vec<_> = balls
                    .iter()
                    .copied()
                    .filter(|ball| {
                        !obstacles
                            .iter()
                            .any(|obstacle| occludes(camera, *ball, obstacle))
                    })
                    .collect();
                let detections = self.detector.detect(camera, &visible, radius, parameters);
                self.detections
                    .announce(time)
                    .await?
                    .publish(&TimeWrapper {
                        time,
                        inner: detections,
                    })
                    .await?;
                self.last_frame = Some(time);
            }
            Ok(())
        })
    }
}

impl Drop for PerceptionIo {
    fn drop(&mut self) {
        self.metrics_task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{Isometry3, vector};
    use ros_z::{qos::QosDurability, time::Clock};

    fn camera() -> CameraMatrix {
        use std::f32::consts::FRAC_PI_2;
        CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            vector![640.0, 544.0],
            Isometry3::identity(),
            Isometry3::identity(),
            Isometry3::wrap(
                nalgebra::Isometry3::rotation(nalgebra::Vector3::y() * -FRAC_PI_2)
                    * nalgebra::Isometry3::rotation(nalgebra::Vector3::x() * FRAC_PI_2)
                    * nalgebra::Isometry3::translation(0.0, 0.0, -0.8),
            ),
        )
    }

    fn clean() -> Parameters {
        Parameters {
            center_noise_pixels: 0.0,
            false_positive_probability: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn occlusion_requires_an_obstacle_between_camera_and_ball_at_ray_height() {
        let camera = camera();
        let ball = point![3.0, 0.0, 0.105];
        let mut obstacle = Occluder {
            center: point![1.5, 0.0, 0.425],
            radius: 0.22,
            height: 0.85,
        };
        assert!(occludes(&camera, ball, &obstacle));
        obstacle.center = point![4.0, 0.0, 0.425];
        assert!(!occludes(&camera, ball, &obstacle));
        obstacle.center = point![1.5, 1.0, 0.425];
        assert!(!occludes(&camera, ball, &obstacle));
        obstacle.center = point![1.5, 0.0, 2.0];
        assert!(!occludes(&camera, ball, &obstacle));
    }

    #[test]
    fn clean_detections_round_trip_and_respect_camera_view() {
        let camera = camera();
        let balls = [
            point![2.0, 0.2, 0.105],
            point![-2.0, 0.0, 0.105],
            point![2.0, 20.0, 0.105],
        ];
        let detections = Detector::new(42).detect(&camera, &balls, 0.105, &clean());
        assert_eq!(detections.len(), 1);
        let position = camera
            .pixel_to_ground_with_z(detections[0].bounding_box.area.center(), 0.105)
            .unwrap();
        assert!((position - balls[0].xy()).norm() < 1e-5);
        let airborne = point![2.0, 0.2, 0.5];
        let detection = Detector::new(42).detect(&camera, &[airborne], 0.105, &clean());
        assert!(
            (detection[0].bounding_box.area.center()
                - camera
                    .ground_with_z_to_pixel(airborne.xy(), airborne.z())
                    .unwrap())
            .norm()
                < 1e-4
        );
    }

    #[test]
    fn seeded_noise_is_repeatable_and_has_requested_variance() {
        let camera = camera();
        let ball = point![2.0, 0.0, 0.105];
        let center = camera.ground_with_z_to_pixel(ball.xy(), ball.z()).unwrap();
        let parameters = Parameters {
            center_noise_pixels: 3.0,
            ..clean()
        };
        let mut first = Detector::new(42);
        let mut second = Detector::new(42);
        let mut sum = nalgebra::Vector2::<f64>::zeros();
        let mut squares = nalgebra::Vector2::<f64>::zeros();
        for _ in 0..10_000 {
            let a = first.detect(&camera, &[ball], 0.105, &parameters);
            let b = second.detect(&camera, &[ball], 0.105, &parameters);
            assert_eq!(
                a[0].bounding_box.area.center(),
                b[0].bounding_box.area.center()
            );
            let residual = (a[0].bounding_box.area.center() - center)
                .inner
                .cast::<f64>();
            sum += residual;
            squares += residual.component_mul(&residual);
        }
        assert!((sum / 10_000.0).amax() < 0.1);
        assert!((squares / 10_000.0 - nalgebra::Vector2::repeat(9.0)).amax() < 0.4);
    }

    #[test]
    fn dropout_and_false_detection_bursts_persist_for_the_configured_frames() {
        let camera = camera();
        let ball = point![2.0, 0.0, 0.105];
        let mut detector = Detector::new(42);
        let mut parameters = Parameters {
            dropout_burst_probability: 1.0,
            dropout_burst_frames: 3,
            false_positive_probability: 1.0,
            false_positive_burst_frames: 3,
            ..clean()
        };
        let first = detector.detect(&camera, &[ball], 0.105, &parameters);
        assert_eq!(first.len(), 1); // True ball dropped, false ball remains.
        parameters.dropout_burst_probability = 0.0;
        parameters.false_positive_probability = 0.0;
        for _ in 0..2 {
            let frame = detector.detect(&camera, &[ball], 0.105, &parameters);
            assert_eq!(frame.len(), 1);
            assert_eq!(
                frame[0].bounding_box.area.center(),
                first[0].bounding_box.area.center()
            );
        }
        let recovered = detector.detect(&camera, &[ball], 0.105, &parameters);
        assert_eq!(recovered.len(), 1);
        let projected = camera
            .pixel_to_ground_with_z(recovered[0].bounding_box.area.center(), 0.105)
            .unwrap();
        assert!((projected - ball.xy()).norm() < 1e-5);
    }

    #[test]
    fn constant_center_bias_is_applied_and_invalid_bursts_are_rejected() {
        let camera = camera();
        let ball = point![2.0, 0.0, 0.105];
        let mut parameters = Parameters {
            center_bias_pixels: [3.0, -2.0],
            ..clean()
        };
        let detection = Detector::new(42).detect(&camera, &[ball], 0.105, &parameters);
        let center = camera.ground_with_z_to_pixel(ball.xy(), ball.z()).unwrap();
        assert!(
            (detection[0].bounding_box.area.center() - center - vector![3.0, -2.0]).norm() < 1e-4
        );
        parameters.false_positive_burst_frames = 0;
        assert!(parameters.validate().is_err());
        parameters.false_positive_burst_frames = 1;
        parameters.dropout_probability = f32::NAN;
        assert!(parameters.validate().is_err());
    }

    #[test]
    fn false_detections_also_occur_without_real_balls() {
        let camera = camera();
        let mut detector = Detector::new(42);
        assert!(detector.detect(&camera, &[], 0.105, &clean()).is_empty());
        let parameters = Parameters {
            false_positive_probability: 1.0,
            ..clean()
        };
        let detections = detector.detect(&camera, &[], 0.105, &parameters);
        assert_eq!(detections.len(), 1);
        assert!(in_image(&camera, detections[0].bounding_box.area.center()));
        assert_eq!(detections[0].label, RobocupObjectLabel::Ball);
        assert!(
            detector
                .detect(&camera, &[point![-2.0, 0.0, 0.105]], 0.105, &clean())
                .is_empty()
        );
    }

    #[test]
    fn metrics_include_misses_and_false_tracks() {
        let mut metrics = Metrics::default();
        let truth = [point![2.0, 0.0, 0.105]];
        let estimate = BallPosition {
            position: point![2.3, 0.4],
            velocity: vector![0.0, 0.0],
            last_seen: Time::zero(),
        };
        metrics.observe(Time::zero(), &truth, Some(estimate));
        assert!((metrics.position_rmse_metres.unwrap() - 0.5).abs() < 1e-6);
        metrics.observe(Time::zero(), &truth, None);
        metrics.observe(Time::zero(), &[], Some(estimate));
        metrics.observe(Time::zero(), &[], None);
        assert_eq!(
            (
                metrics.matched_samples,
                metrics.missing_estimates,
                metrics.estimates_without_ball,
                metrics.empty_samples
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(metrics.position_error_metres, None);
    }

    #[test]
    fn field_transform_error_is_separate_from_primary_ball_error() {
        let time = Time::from_nanos(200_000_000);
        let truth = TruthFrame {
            balls: vec![point![2.0, 0.0, 0.105]],
            ground_to_field: Isometry2::identity(),
        };
        let estimate = Some(BallPosition {
            position: point![2.0, 0.0],
            velocity: vector![0.0, 0.0],
            last_seen: time,
        });
        let mut metrics = Metrics::default();
        metrics.observe(time, &truth.balls, estimate);
        let incorrect_pose =
            Isometry2::wrap(nalgebra::Isometry2::new(nalgebra::vector![0.3, 0.4], 0.0));
        metrics.observe_field(&truth, estimate, Some((time, incorrect_pose)));
        assert_eq!(metrics.position_rmse_metres, Some(0.0));
        assert!((metrics.field_position_rmse_metres.unwrap() - 0.5).abs() < 1e-6);
        metrics.observe_field(&truth, estimate, Some((Time::zero(), incorrect_pose)));
        assert_eq!(metrics.missing_field_transforms, 1);
        assert_eq!(metrics.field_position_error_metres, None);
        assert_eq!(metrics.position_rmse_metres, Some(0.0));
    }

    #[test]
    fn rejects_invalid_noise_configuration() {
        assert!(Parameters::default().validate().is_ok());
        for value in [-1.0, f32::NAN, f32::INFINITY] {
            assert!(
                Parameters {
                    center_noise_pixels: value,
                    ..clean()
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            Parameters {
                false_positive_probability: 1.1,
                ..clean()
            }
            .validate()
            .is_err()
        );
        assert!(
            Parameters {
                frame_period: 0.0,
                ..clean()
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn production_filter_consumes_announcements_and_metrics_match_capture_time() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let clock = Clock::logical(Time::zero());
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("tcp/127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);
        let (
            context,
            node,
            mut perception,
            camera_pub,
            metrics,
            estimates,
            task,
            odometry,
            pose_pub,
        ) = runtime.block_on(async {
            let context = Arc::new(
                ContextBuilder::default()
                    .with_namespace("/ball_perception_test")
                    .with_mode("router")
                    .disable_multicast_scouting()
                    .with_connect_endpoints(std::iter::empty::<&str>())
                    .with_listen_endpoints([endpoint.as_str()])
                    .with_parameter_layers([std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../../etc/parameters/base")])
                    .with_clock(clock.clone())
                    .build()
                    .await
                    .unwrap(),
            );
            let node = context.create_node("test_io").build().await.unwrap();
            let perception = PerceptionIo::new(&node, runtime.handle()).await.unwrap();
            let field = node
                .publisher::<types::field_dimensions::FieldDimensions>("field_dimensions")
                .qos(QosProfile {
                    durability: QosDurability::TransientLocal,
                    ..Default::default()
                })
                .build()
                .await
                .unwrap();
            field
                .publish(&types::field_dimensions::FieldDimensions {
                    ball_radius: 0.105,
                    ..types::field_dimensions::FieldDimensions::SPL_2025
                })
                .await
                .unwrap();
            let camera_pub = node
                .publisher::<TimeWrapper<CameraMatrix>>("camera_matrix")
                .build()
                .await
                .unwrap();
            let metrics = node
                .subscriber::<Metrics>("simulation/ball_filter_metrics")
                .cache(1)
                .build()
                .await
                .unwrap();
            let estimates = node
                .subscriber::<Option<BallPosition<Ground>>>("ball_filter/ball_position")
                .cache(1)
                .build()
                .await
                .unwrap();
            let odometry = node
                .announcing_publisher::<linear_algebra::Pose2<coordinate_systems::Odometry>>(
                    "inputs/odometry",
                )
                .await
                .unwrap();
            let pose_pub = node
                .publisher::<Isometry2<Ground, Field>>("ground_to_field")
                .build()
                .await
                .unwrap();
            let task = tokio::spawn(ball_filter::run_boxed(context.clone()));
            // Keep the retained publisher alive throughout the test.
            (
                context,
                (node, field),
                perception,
                camera_pub,
                metrics,
                estimates,
                task,
                odometry,
                pose_pub,
            )
        });
        let camera = camera();
        for tick in 1..=120 {
            let time = Time::from_nanos(tick * 20_000_000);
            clock.set_time(time).unwrap();
            runtime
                .block_on(camera_pub.publish(&TimeWrapper {
                    time,
                    inner: camera.clone(),
                }))
                .unwrap();
            // The robot translates and turns; a stationary world ball changes Ground coordinates.
            let pose = nalgebra::Isometry3::new(
                nalgebra::vector![tick as f32 * 0.001, 0.0, 0.0],
                nalgebra::vector![0.0, 0.0, tick as f32 * 0.0005],
            );
            let ball = Point3::wrap(pose.inverse() * nalgebra::point![2.0, 0.0, 0.105]);
            let ground_to_field = crate::behavior_inputs::ground_to_field(
                pose,
                types::field_dimensions::GlobalFieldSide::Home,
            );
            // Register truth before the filter can receive either input at this time.
            perception
                .publish(time, ground_to_field, &camera, vec![ball], 0.105, &clean())
                .unwrap();
            runtime.block_on(async {
                pose_pub
                    .publish_with_source_time(&ground_to_field, time)
                    .await
                    .unwrap();
                odometry
                    .announce(time)
                    .await
                    .unwrap()
                    .publish(&linear_algebra::Pose2::new(
                        point![pose.translation.x, pose.translation.y],
                        pose.rotation.euler_angles().2,
                    ))
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
            });
            assert!(!task.is_finished(), "production filter exited");
        }
        let result = metrics
            .get_latest()
            .expect("production filter did not produce metrics");
        assert!(result.matched_samples > 10, "{result:?}");
        assert_eq!(result.unmatched_timestamps, 0, "{result:?}");
        assert!(result.position_rmse_metres.unwrap() < 0.15, "{result:?}");
        assert!(estimates.get_latest().unwrap().is_some());
        assert!(result.field_matched_samples > 10, "{result:?}");
        assert!(
            result.field_position_rmse_metres.unwrap() < 0.15,
            "{result:?}"
        );
        task.abort();
        runtime.block_on(async {
            let _ = task.await;
        });
        drop(perception);
        drop(node);
        context.shutdown().unwrap();
    }
}
