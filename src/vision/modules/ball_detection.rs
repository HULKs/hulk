use compiled_nn::CompiledNN;
use module_derive::{module, require_some};
use nalgebra::{point, vector};
use types::{
    Ball, CameraMatrix, CandidateEvaluation, Circle, PerspectiveGridCandidates, Rectangle,
};

use crate::framework::configuration::BallDetection as BallDetectionConfiguration;

pub const SAMPLE_SIZE: usize = 32;
pub type Sample = [[f32; SAMPLE_SIZE]; SAMPLE_SIZE];

struct NeuralNetworks {
    preclassifier: CompiledNN,
    classifier: CompiledNN,
    positioner: CompiledNN,
}

unsafe impl Send for NeuralNetworks {}

#[derive(Debug)]
struct BallCluster<'a> {
    circle: Circle,
    members: Vec<&'a CandidateEvaluation>,
}

pub struct BallDetection {
    neural_networks: Option<NeuralNetworks>,
}

#[module(vision)]
#[input(path = perspective_grid_candidates, data_type = PerspectiveGridCandidates)]
#[input(path = camera_matrix, data_type = CameraMatrix)]
#[parameter(path = $this_cycler.ball_detection, data_type = BallDetectionConfiguration)]
#[parameter(path = field_dimensions.ball_radius, data_type = f32, name = ball_radius)]
#[additional_output(path = ball_candidates, data_type = Vec<CandidateEvaluation>)]
#[main_output(name = balls, data_type = Vec<Ball>)]
impl BallDetection {}

impl BallDetection {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            neural_networks: Default::default(),
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let candidates = &require_some!(context.perspective_grid_candidates).candidates;
        let camera_matrix = require_some!(context.camera_matrix);
        let networks = self.get_neural_networks(&context);

        let evaluations = Self::evaluate_candidates(
            candidates,
            context.image,
            networks,
            context
                .ball_detection
                .maximum_number_of_candidate_evaluations,
            context.ball_detection.ball_radius_enlargement_factor,
            context.ball_detection.preclassifier_confidence_threshold,
            context.ball_detection.classifier_confidence_threshold,
        );
        context
            .ball_candidates
            .fill_on_subscription(|| evaluations.clone());

        let mut detected_balls = evaluations
            .iter()
            .filter(|candidate| candidate.corrected_circle.is_some())
            .cloned()
            .collect::<Vec<_>>();

        for ball in &mut detected_balls {
            ball.merge_weight = Some(Self::calculate_ball_merge_factor(
                ball,
                (context.image.width(), context.image.height()),
                context.ball_detection.confidence_merge_factor,
                context.ball_detection.correction_proximity_merge_factor,
                context.ball_detection.image_containment_merge_factor,
            ));
        }

        let clusters = Self::cluster_balls(
            &detected_balls,
            context.ball_detection.cluster_merge_radius_factor,
        );

        let balls = Self::project_balls_to_ground(&clusters, camera_matrix, *context.ball_radius);

        Ok(MainOutputs { balls: Some(balls) })
    }

    fn get_neural_networks(&mut self, context: &CycleContext) -> &mut NeuralNetworks {
        if self.neural_networks.is_none() {
            let mut preclassifier = CompiledNN::default();
            preclassifier.compile(context.ball_detection.preclassifier_neural_network.clone());

            let mut classifier = CompiledNN::default();
            classifier.compile(context.ball_detection.classifier_neural_network.clone());

            let mut positioner = CompiledNN::default();
            positioner.compile(context.ball_detection.positioner_neural_network.clone());

            self.neural_networks = Some(NeuralNetworks {
                preclassifier,
                classifier,
                positioner,
            });
        }

        self.neural_networks.as_mut().unwrap()
    }

    fn preclassify_sample(network: &mut CompiledNN, sample: &Sample) -> f32 {
        let input = network.input(0);
        for y in 0..SAMPLE_SIZE {
            for x in 0..SAMPLE_SIZE {
                input[x + y * SAMPLE_SIZE] = sample[y][x];
            }
        }
        network.apply();
        network.output(0)[0]
    }

    fn classify_sample(network: &mut CompiledNN, sample: &Sample) -> f32 {
        let input = network.input(0);
        for y in 0..SAMPLE_SIZE {
            for x in 0..SAMPLE_SIZE {
                input[x + y * SAMPLE_SIZE] = sample[y][x];
            }
        }
        network.apply();
        network.output(0)[0]
    }

    fn position_sample(network: &mut CompiledNN, sample: &Sample) -> Circle {
        let input = network.input(0);
        for y in 0..SAMPLE_SIZE {
            for x in 0..SAMPLE_SIZE {
                input[x + y * SAMPLE_SIZE] = sample[y][x];
            }
        }
        network.apply();
        Circle {
            center: point![network.output(0)[0], network.output(0)[1]],
            radius: network.output(0)[2],
        }
    }

    fn sample_grayscale(image: &Image422, candidate: Circle) -> Sample {
        let top_left = candidate.center - vector![candidate.radius, candidate.radius];
        let image_pixels_per_sample_pixel = candidate.radius * 2.0 / SAMPLE_SIZE as f32;

        let mut sample = Sample::default();
        for (y, column) in sample.iter_mut().enumerate() {
            for (x, pixel) in column.iter_mut().enumerate() {
                let sample_point = point![
                    (top_left.x + x as f32 * image_pixels_per_sample_pixel) * 0.5,
                    top_left.y + y as f32 * image_pixels_per_sample_pixel
                ];
                *pixel = image.try_at(sample_point).map_or(
                    128.0,
                    |color| if x % 2 == 0 { color.y1 } else { color.y2 } as f32,
                );
            }
        }

        sample
    }

    fn evaluate_candidates(
        candidates: &[Circle],
        image: &Image422,
        networks: &mut NeuralNetworks,
        maximum_number_of_candidate_evaluations: usize,
        ball_radius_enlargement_factor: f32,
        classifier_confidence_threshold: f32,
        preclassifier_confidence_threshold: f32,
    ) -> Vec<CandidateEvaluation> {
        let preclassifier = &mut networks.preclassifier;
        let classifier = &mut networks.classifier;
        let positioner = &mut networks.positioner;

        candidates
            .iter()
            .take(maximum_number_of_candidate_evaluations)
            .map(|candidate| {
                let enlarged_candidate = Circle {
                    center: candidate.center,
                    radius: candidate.radius * ball_radius_enlargement_factor,
                };
                let sample = Self::sample_grayscale(image, enlarged_candidate);
                let preclassifier_confidence = Self::preclassify_sample(preclassifier, &sample);

                let mut classifier_confidence = None;
                if preclassifier_confidence > preclassifier_confidence_threshold {
                    classifier_confidence = Some(Self::classify_sample(classifier, &sample))
                };

                let mut corrected_circle = None;
                if classifier_confidence > Some(classifier_confidence_threshold) {
                    let raw_corrected_circle = Self::position_sample(positioner, &sample);

                    corrected_circle = Some(Circle {
                        center: candidate.center
                            + (raw_corrected_circle.center.coords - vector![0.5, 0.5])
                                * (candidate.radius * 2.0)
                                * ball_radius_enlargement_factor,
                        radius: raw_corrected_circle.radius
                            * candidate.radius
                            * ball_radius_enlargement_factor,
                    });
                }

                CandidateEvaluation {
                    candidate_circle: *candidate,
                    preclassifier_confidence,
                    classifier_confidence,
                    corrected_circle,
                    merge_weight: None,
                }
            })
            .collect()
    }

    fn bounding_box_patch_intersection(circle: Circle, patch_candidate_circle: Circle) -> f32 {
        let patch = patch_candidate_circle.bounding_box();
        let circle_box = circle.bounding_box();

        let intersection_area = circle_box.rectangle_intersection(patch);
        intersection_area / circle_box.area()
    }

    fn image_containment(circle: Circle, image_size: (usize, usize)) -> f32 {
        let image_rectangle = Rectangle {
            top_left: point![0.0, 0.0],
            bottom_right: point![image_size.0 as f32 * 2.0, image_size.1 as f32],
        };
        let circle_box = circle.bounding_box();

        let intersection_area = circle_box.rectangle_intersection(image_rectangle);
        intersection_area / circle_box.area()
    }

    fn calculate_ball_merge_factor(
        ball: &CandidateEvaluation,
        image_size: (usize, usize),
        confidence_merge_factor: f32,
        correction_proximity_merge_factor: f32,
        image_containment_merge_factor: f32,
    ) -> f32 {
        let confidence = ball.classifier_confidence.unwrap();
        let correction_proximity = Self::bounding_box_patch_intersection(
            ball.corrected_circle.unwrap(),
            ball.candidate_circle,
        );
        let image_containment = Self::image_containment(ball.corrected_circle.unwrap(), image_size);

        confidence.powf(confidence_merge_factor)
            * correction_proximity.powf(correction_proximity_merge_factor)
            * image_containment.powf(image_containment_merge_factor)
    }

    fn merge_balls(balls: &[&CandidateEvaluation]) -> Circle {
        let mut circle = Circle {
            center: point![0.0, 0.0],
            radius: 0.0,
        };

        let total_weight: f32 = balls.iter().map(|ball| ball.merge_weight.unwrap()).sum();
        for ball in balls {
            let ball_circle = ball.corrected_circle.unwrap();
            let weight = ball.merge_weight.unwrap();
            circle.center += ball_circle.center.coords * weight / total_weight;
            circle.radius += ball_circle.radius * weight / total_weight;
        }

        circle
    }

    fn cluster_balls(balls: &[CandidateEvaluation], merge_radius_factor: f32) -> Vec<BallCluster> {
        let mut clusters = Vec::<BallCluster>::new();

        for ball in balls {
            let ball_circle = ball.corrected_circle.unwrap();
            match clusters.iter_mut().find(|cluster| {
                (cluster.circle.center - ball_circle.center).norm_squared()
                    < (cluster.circle.radius * merge_radius_factor).powi(2)
            }) {
                Some(cluster) => {
                    cluster.members.push(ball);
                    cluster.circle = Self::merge_balls(cluster.members.as_slice());
                }
                None => clusters.push(BallCluster {
                    circle: ball_circle,
                    members: vec![ball],
                }),
            }
        }

        clusters
    }

    fn project_balls_to_ground(
        clusters: &[BallCluster],
        camera_matrix: &CameraMatrix,
        ball_radius: f32,
    ) -> Vec<Ball> {
        clusters
            .iter()
            .filter_map(|cluster| {
                let position_422 = point![cluster.circle.center.x, cluster.circle.center.y];
                match camera_matrix.pixel_to_ground_with_z(&position_422, ball_radius) {
                    Ok(position) => Some(Ball {
                        position,
                        image_location: cluster.circle,
                    }),
                    Err(_) => None,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use anyhow::anyhow;
    use approx::assert_relative_eq;
    use nalgebra::{Isometry3, Translation, UnitQuaternion};
    use types::CameraPosition;

    use crate::framework::AdditionalOutput;

    use super::*;

    const PRECLASSIFIER_PATH: &str = "etc/neural_networks/preclassifier.hdf5";
    const CLASSIFIER_PATH: &str = "etc/neural_networks/classifier.hdf5";
    const POSITIONER_PATH: &str = "etc/neural_networks/positioner.hdf5";

    const BALL_SAMPLE_PATH: &str = "tests/data/ball_sample.png";

    #[test]
    fn preclassify_ball() {
        let mut network = CompiledNN::default();
        network.compile(CLASSIFIER_PATH);
        let sample = BallDetection::sample_grayscale(
            &Image422::load_from_ycbcr_444_file(Path::new(BALL_SAMPLE_PATH)).unwrap(),
            Circle {
                center: point![16.0, 16.0],
                radius: 16.0,
            },
        );
        let confidence = BallDetection::preclassify_sample(&mut network, &sample);

        println!("{:?}", confidence);
        assert_relative_eq!(confidence, 1.0, epsilon = 0.01);
    }

    #[test]
    fn classify_ball() {
        let mut network = CompiledNN::default();
        network.compile(PRECLASSIFIER_PATH);
        let sample = BallDetection::sample_grayscale(
            &Image422::load_from_ycbcr_444_file(Path::new(BALL_SAMPLE_PATH)).unwrap(),
            Circle {
                center: point![16.0, 16.0],
                radius: 16.0,
            },
        );
        let confidence = BallDetection::classify_sample(&mut network, &sample);

        println!("{:?}", confidence);
        assert_relative_eq!(confidence, 1.0, epsilon = 0.01);
    }

    #[test]
    fn position_ball() {
        let mut network = CompiledNN::default();
        network.compile(POSITIONER_PATH);
        let sample = BallDetection::sample_grayscale(
            &Image422::load_from_ycbcr_444_file(Path::new(BALL_SAMPLE_PATH)).unwrap(),
            Circle {
                center: point![16.0, 16.0],
                radius: 16.0,
            },
        );
        let circle = BallDetection::position_sample(&mut network, &sample);

        assert_relative_eq!(
            circle,
            Circle {
                center: point![0.488, 0.514],
                radius: 0.6311
            },
            epsilon = 0.01
        )
    }

    #[test]
    fn candidate_evaluation_simple() {
        let ball_candidate = CandidateEvaluation {
            candidate_circle: Circle {
                center: point![50.0, 50.0],
                radius: 32.0,
            },
            preclassifier_confidence: 1.0,
            classifier_confidence: Some(1.0),
            corrected_circle: Some(Circle {
                center: point![50.0, 50.0],
                radius: 32.0,
            }),
            merge_weight: None,
        };
        let merge_weight =
            BallDetection::calculate_ball_merge_factor(&ball_candidate, (45, 90), 1.0, 1.0, 1.0);
        assert_relative_eq!(merge_weight, 1.0);
    }

    #[test]
    fn candidate_evaluation_complex() {
        let ball_candidate = CandidateEvaluation {
            candidate_circle: Circle {
                center: point![50.0, 50.0],
                radius: 32.0,
            },
            preclassifier_confidence: 1.0,
            classifier_confidence: Some(0.5),
            corrected_circle: Some(Circle {
                center: point![66.0, 50.0],
                radius: 32.0,
            }),
            merge_weight: None,
        };
        let merge_weight =
            BallDetection::calculate_ball_merge_factor(&ball_candidate, (45, 90), 1.0, 1.0, 1.0);
        assert_relative_eq!(merge_weight, 0.5 * 0.75 * (7.0 / 8.0));
    }

    #[test]
    fn cycle_with_loaded_image() -> anyhow::Result<()> {
        let filename = "tests/data/rome_bottom_ball.png";
        let image = Image422::load_from_ycbcr_444_file(Path::new(filename))?;
        let ball_detection_config = BallDetectionConfiguration {
            minimal_radius: 0.0,
            preclassifier_neural_network: PathBuf::from(PRECLASSIFIER_PATH),
            classifier_neural_network: PathBuf::from(CLASSIFIER_PATH),
            positioner_neural_network: PathBuf::from(POSITIONER_PATH),
            maximum_number_of_candidate_evaluations: 75,
            preclassifier_confidence_threshold: 0.9,
            classifier_confidence_threshold: 0.9,
            confidence_merge_factor: 1.0,
            correction_proximity_merge_factor: 1.0,
            image_containment_merge_factor: 1.0,
            cluster_merge_radius_factor: 1.5,
            ball_radius_enlargement_factor: 2.0,
        };
        let perspective_grid_candidates = PerspectiveGridCandidates {
            candidates: vec![Circle {
                center: point![343.0, 184.0],
                radius: 36.0,
            }],
        };

        let focal_length = vector![0.95, 1.27];
        let optical_center = point![0.5, 0.5];

        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            focal_length,
            optical_center,
            vector![image.width() as f32, image.height() as f32],
            Isometry3 {
                rotation: UnitQuaternion::from_euler_angles(0.0, 39.7_f32.to_radians(), 0.0),
                translation: Translation::from(point![0.0, 0.0, 0.75]),
            },
            Isometry3::identity(),
            Isometry3::identity(),
        );

        let mut additional_output_buffer = None;
        let context = CycleContext {
            ball_candidates: AdditionalOutput::<Vec<CandidateEvaluation>>::new(
                false,
                &mut additional_output_buffer,
            ),
            ball_detection: &ball_detection_config,
            ball_radius: &0.5,
            camera_matrix: &Some(camera_matrix),
            camera_position: CameraPosition::Bottom,
            image: &image,
            perspective_grid_candidates: &Some(perspective_grid_candidates),
        };
        let mut ball_detection_neural_net = BallDetection {
            neural_networks: Default::default(),
        };
        let balls = ball_detection_neural_net
            .cycle(context)?
            .balls
            .ok_or_else(|| anyhow!("No result returned"))?;

        assert_relative_eq!(
            balls[0],
            Ball {
                position: point![0.376, -0.22],
                image_location: Circle {
                    center: point![307.7, 175.16],
                    radius: 43.12,
                }
            },
            epsilon = 0.01,
        );
        Ok(())
    }
}
