use color_eyre::Result;
use compiled_nn::CompiledNN;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use itertools::Itertools;
use nalgebra::{point, vector, Vector2};
use types::{
    configuration::BallDetection as BallDetectionConfiguration, image::Image, Ball, CameraMatrix,
    CandidateEvaluation, Circle, Feet, PenaltySpot, PerspectiveGridCandidates, Rectangle,
    RobotPart,
};

pub const SAMPLE_SIZE: usize = 32;
pub type Sample = [[f32; SAMPLE_SIZE]; SAMPLE_SIZE];

struct NeuralNetworks {
    preclassifier: CompiledNN,
    classifier: CompiledNN,
    positioner: CompiledNN,
}

pub struct ClassConfidences {
    ball: f32,
    feet: f32,
    robot_part: f32,
    penalty_spot: f32,
    other: f32,
}

#[derive(Clone, Copy)]
enum DetectableClass {
    Ball,
    Feet,
    RobotPart,
    PenaltySpot,
    Other,
}

#[derive(Clone, Copy)]
struct DetectedClass {
    class: DetectableClass,
    confidence: f32,
}

unsafe impl Send for NeuralNetworks {}

struct BallCluster<'a> {
    circle: Circle,
    members: Vec<&'a CandidateEvaluation>,
}

pub struct BallDetection {
    neural_networks: NeuralNetworks,
}

#[context]
pub struct CreationContext {
    pub configuration: Parameter<BallDetectionConfiguration, "ball_detection.$cycler_instance">,
}

#[context]
pub struct CycleContext {
    pub object_candidates: AdditionalOutput<Vec<CandidateEvaluation>, "object_candidates">,

    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub perspective_grid_candidates:
        RequiredInput<Option<PerspectiveGridCandidates>, "perspective_grid_candidates?">,
    pub image: Input<Image, "image">,

    pub configuration: Parameter<BallDetectionConfiguration, "ball_detection.$cycler_instance">,
    pub ball_radius: Parameter<f32, "field_dimensions.ball_radius">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub balls: MainOutput<Option<Vec<Ball>>>,
    pub feet: MainOutput<Option<Vec<Ball>>>,
    pub robot_parts: MainOutput<Option<Vec<Ball>>>,
    pub penalty_spot: MainOutput<Option<Vec<Ball>>>,
}

impl DetectedClass {
    fn new(class: DetectableClass, confidence: f32) -> Self {
        DetectedClass { class, confidence }
    }
}

impl BallDetection {
    pub fn new(context: CreationContext) -> Result<Self> {
        let mut preclassifier = CompiledNN::default();
        preclassifier.compile(&context.configuration.preclassifier_neural_network);

        let mut classifier = CompiledNN::default();
        classifier.compile(&context.configuration.classifier_neural_network);

        let mut positioner = CompiledNN::default();
        positioner.compile(&context.configuration.positioner_neural_network);

        let neural_networks = NeuralNetworks {
            preclassifier,
            classifier,
            positioner,
        };
        Ok(Self { neural_networks })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let candidates = &context.perspective_grid_candidates.candidates;

        let evaluations = evaluate_candidates(
            candidates,
            context.image,
            &mut self.neural_networks,
            context
                .configuration
                .maximum_number_of_candidate_evaluations,
            context.configuration.ball_radius_enlargement_factor,
            context.configuration.classifier_confidence_threshold,
            context.configuration.preclassifier_confidence_threshold,
        );

        context
            .object_candidates
            .fill_if_subscribed(|| evaluations.clone());

        let balls = collect_balls(&context, &evaluations);
        let feet = collect_feet(&context, &evaluations);
        let robot_parts = collect_robot_parts(&context, &evaluations);
        let penalty_spot = collect_penalty_spots(&context, &evaluations);

        Ok(MainOutputs {
            balls: Some(balls).into(),
            feet: Some(feet).into(),
            robot_parts: Some(robot_parts).into(),
            penalty_spot: Some(penalty_spot).into(),
        })
    }
}

fn collect_balls(context: &CycleContext, evaluations: &[CandidateEvaluation]) -> Vec<Ball> {
    let mut detected_balls = evaluations
        .iter()
        .filter(|candidate| candidate.positioned_ball.is_some())
        .cloned()
        .collect::<Vec<_>>();

    for ball in &mut detected_balls {
        ball.merge_weight = Some(calculate_ball_merge_factor(
            ball,
            vector!(context.image.width(), context.image.height()),
            context.configuration.confidence_merge_factor,
            context.configuration.correction_proximity_merge_factor,
            context.configuration.image_containment_merge_factor,
        ));
    }

    let clusters = cluster_balls(
        &detected_balls,
        context.configuration.cluster_merge_radius_factor,
    );

    project_balls_to_ground(&clusters, context.camera_matrix, *context.ball_radius)
}

fn collect_feet(context: &CycleContext, evaluations: &[CandidateEvaluation]) -> Vec<Ball> {
    let mut detected_feet = evaluations
        .iter()
        .filter(|candidate| candidate.positioned_feet.is_some())
        .cloned()
        .collect::<Vec<_>>();

    let clusters = cluster_feet(
        &detected_feet,
        context.configuration.cluster_merge_radius_factor,
    );
    
    project_balls_to_ground(&clusters, context.camera_matrix, *context.ball_radius)
}

fn collect_robot_parts(context: &CycleContext, evaluations: &[CandidateEvaluation]) -> Vec<Ball> {
    let mut detected_robot_parts = evaluations
        .iter()
        .filter(|candidate| candidate.positioned_robot_part.is_some())
        .cloned()
        .collect::<Vec<_>>();

    let clusters = cluster_robot_parts(
        &detected_robot_parts,
        context.configuration.cluster_merge_radius_factor,
    );
    
    project_balls_to_ground(&clusters, context.camera_matrix, *context.ball_radius)
}

fn collect_penalty_spots(context: &CycleContext, evaluations: &[CandidateEvaluation]) -> Vec<Ball> {
    let mut detected_penalty_spots = evaluations
        .iter()
        .filter(|candidate| candidate.positioned_penalty_spot.is_some())
        .cloned()
        .collect::<Vec<_>>();

    let clusters = cluster_penalty_spots(
        &detected_penalty_spots,
        context.configuration.cluster_merge_radius_factor,
    );
    
    project_balls_to_ground(&clusters, context.camera_matrix, *context.ball_radius)
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

fn classify_sample(network: &mut CompiledNN, sample: &Sample) -> ClassConfidences {
    let input = network.input(0);
    for y in 0..SAMPLE_SIZE {
        for x in 0..SAMPLE_SIZE {
            input[x + y * SAMPLE_SIZE] = sample[y][x];
        }
    }
    network.apply();
    ClassConfidences {
        ball: network.output(0)[0],
        feet: network.output(0)[1],
        robot_part: network.output(0)[2],
        penalty_spot: network.output(0)[3],
        other: network.output(0)[4],
    }
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

fn sample_grayscale(image: &Image, candidate: Circle) -> Sample {
    let top_left = candidate.center - vector![candidate.radius, candidate.radius];
    let image_pixels_per_sample_pixel = candidate.radius * 2.0 / SAMPLE_SIZE as f32;

    let mut sample = Sample::default();
    for (y, column) in sample.iter_mut().enumerate() {
        for (x, pixel) in column.iter_mut().enumerate() {
            let x = (top_left.x + x as f32 * image_pixels_per_sample_pixel) as u32;
            let y = (top_left.y + y as f32 * image_pixels_per_sample_pixel) as u32;
            *pixel = image.try_at(x, y).map_or(128.0, |pixel| pixel.y as f32);
        }
    }

    sample
}

fn evaluate_candidates(
    candidates: &[Circle],
    image: &Image,
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
            let sample = sample_grayscale(image, enlarged_candidate);
            let preclassifier_confidence = preclassify_sample(preclassifier, &sample);

            let mut detected_class = None;
            if preclassifier_confidence > preclassifier_confidence_threshold {
                let classifier_confidences = classify_sample(classifier, &sample);
                detected_class =
                    decide_detected_class(classifier_confidences, classifier_confidence_threshold);
            }

            let mut positioned_ball = None;
            let mut positioned_feet = None;
            let mut positioned_robot_part = None;
            let mut positioned_penalty_spot = None;

            if let Some(ref detected_class) = detected_class {
                match detected_class.class {
                    DetectableClass::Ball => {
                        let raw_positioned_ball = position_sample(positioner, &sample);

                        positioned_ball = Some(Circle {
                            center: candidate.center
                                + (raw_positioned_ball.center.coords - vector![0.5, 0.5])
                                    * (candidate.radius * 2.0)
                                    * ball_radius_enlargement_factor,
                            radius: raw_positioned_ball.radius
                                * candidate.radius
                                * ball_radius_enlargement_factor,
                        });
                    }
                    DetectableClass::Feet => {
                        positioned_feet = Some(Circle {
                            center: candidate.center,
                            radius: 50.0,
                        })
                    }
                    DetectableClass::RobotPart => {
                        positioned_robot_part = Some(Circle {
                            center: candidate.center,
                            radius: 60.0,
                        })
                    }
                    DetectableClass::PenaltySpot => {
                        positioned_penalty_spot = Some(Circle {
                            center: candidate.center,
                            radius: 70.0,
                        })
                    }
                    DetectableClass::Other => {}
                }
            }

            let classifier_confidence = detected_class.map(|dc| dc.confidence);

            CandidateEvaluation {
                grid_element: *candidate,
                preclassifier_confidence,
                classifier_confidence,
                positioned_ball,
                positioned_feet,
                positioned_robot_part,
                positioned_penalty_spot,
                merge_weight: None,
            }
        })
        .collect()
}

fn decide_detected_class(
    classifier_confidences: ClassConfidences,
    classifier_confidence_threshold: f32,
) -> Option<DetectedClass> {
    use DetectableClass::{Ball, Feet, PenaltySpot, RobotPart};
    let confidences = [
        DetectedClass::new(Ball, classifier_confidences.ball),
        DetectedClass::new(Feet, classifier_confidences.feet),
        DetectedClass::new(RobotPart, classifier_confidences.robot_part),
        DetectedClass::new(PenaltySpot, classifier_confidences.penalty_spot),
    ];

    let most_probable_class = confidences
        .iter()
        .position_max_by(|&a, &b| a.confidence.total_cmp(&b.confidence))
        .expect("There are always multiple elements in the confidence array");

    let detected_class = confidences[most_probable_class];
    if detected_class.confidence > classifier_confidence_threshold {
        Some(detected_class)
    } else {
        None
    }
}

fn bounding_box_patch_intersection(circle: Circle, patch_candidate_circle: Circle) -> f32 {
    let patch = patch_candidate_circle.bounding_box();
    let circle_box = circle.bounding_box();

    let intersection_area = circle_box.rectangle_intersection(patch);
    intersection_area / circle_box.area()
}

fn image_containment(circle: Circle, image_size: Vector2<u32>) -> f32 {
    let image_rectangle = Rectangle {
        top_left: point![0.0, 0.0],
        bottom_right: point![image_size.x as f32, image_size.y as f32],
    };
    let circle_box = circle.bounding_box();

    let intersection_area = circle_box.rectangle_intersection(image_rectangle);
    intersection_area / circle_box.area()
}

fn calculate_ball_merge_factor(
    ball: &CandidateEvaluation,
    image_size: Vector2<u32>,
    confidence_merge_factor: f32,
    correction_proximity_merge_factor: f32,
    image_containment_merge_factor: f32,
) -> f32 {
    let confidence = ball.classifier_confidence.unwrap();
    let correction_proximity =
        bounding_box_patch_intersection(ball.positioned_ball.unwrap(), ball.grid_element);
    let image_containment = image_containment(ball.positioned_ball.unwrap(), image_size);

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
        let ball_circle = ball.positioned_ball.unwrap();
        let weight = ball.merge_weight.unwrap();
        circle.center += ball_circle.center.coords * weight / total_weight;
        circle.radius += ball_circle.radius * weight / total_weight;
    }

    circle
}

fn cluster_balls(balls: &[CandidateEvaluation], merge_radius_factor: f32) -> Vec<BallCluster> {
    let mut clusters = Vec::<BallCluster>::new();

    for ball in balls {
        let ball_circle = ball.positioned_ball.unwrap();
        match clusters.iter_mut().find(|cluster| {
            (cluster.circle.center - ball_circle.center).norm_squared()
                < (cluster.circle.radius * merge_radius_factor).powi(2)
        }) {
            Some(cluster) => {
                cluster.members.push(ball);
                cluster.circle = merge_balls(cluster.members.as_slice());
            }
            None => clusters.push(BallCluster {
                circle: ball_circle,
                members: vec![ball],
            }),
        }
    }

    clusters
}

fn cluster_feet(objects: &[CandidateEvaluation], merge_radius_factor: f32) -> Vec<BallCluster> {
    let mut clusters = Vec::<BallCluster>::new();

    for test_object in objects {
        let ball_circle = test_object.positioned_feet.unwrap();
        match clusters.iter_mut().find(|cluster| {
            (cluster.circle.center - ball_circle.center).norm_squared()
                < (cluster.circle.radius * merge_radius_factor).powi(2)
        }) {
            Some(cluster) => {
                cluster.members.push(test_object);
                cluster.circle = merge_balls(cluster.members.as_slice());
            }
            None => clusters.push(BallCluster {
                circle: ball_circle,
                members: vec![test_object],
            }),
        }
    }

    clusters
}

fn cluster_robot_parts(objects: &[CandidateEvaluation], merge_radius_factor: f32) -> Vec<BallCluster> {
    let mut clusters = Vec::<BallCluster>::new();

    for test_object in objects {
        let ball_circle = test_object.positioned_robot_part.unwrap();
        match clusters.iter_mut().find(|cluster| {
            (cluster.circle.center - ball_circle.center).norm_squared()
                < (cluster.circle.radius * merge_radius_factor).powi(2)
        }) {
            Some(cluster) => {
                cluster.members.push(test_object);
                cluster.circle = merge_balls(cluster.members.as_slice());
            }
            None => clusters.push(BallCluster {
                circle: ball_circle,
                members: vec![test_object],
            }),
        }
    }

    clusters
}

fn cluster_penalty_spots(objects: &[CandidateEvaluation], merge_radius_factor: f32) -> Vec<BallCluster> {
    let mut clusters = Vec::<BallCluster>::new();

    for test_object in objects {
        let ball_circle = test_object.positioned_penalty_spot.unwrap();
        match clusters.iter_mut().find(|cluster| {
            (cluster.circle.center - ball_circle.center).norm_squared()
                < (cluster.circle.radius * merge_radius_factor).powi(2)
        }) {
            Some(cluster) => {
                cluster.members.push(test_object);
                cluster.circle = merge_balls(cluster.members.as_slice());
            }
            None => clusters.push(BallCluster {
                circle: ball_circle,
                members: vec![test_object],
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use approx::assert_relative_eq;
    use nalgebra::{Isometry3, Translation, UnitQuaternion};

    use super::*;

    const PRECLASSIFIER_PATH: &str = "../../etc/neural_networks/preclassifier.hdf5";
    const CLASSIFIER_PATH: &str = "../../etc/neural_networks/classifier_multiclass.hdf5";
    const POSITIONER_PATH: &str = "../../etc/neural_networks/positioner.hdf5";

    const BALL_SAMPLE_PATH: &str = "../../tests/data/ball_sample.png";

    #[test]
    fn preclassify_ball() {
        let mut network = CompiledNN::default();
        network.compile(CLASSIFIER_PATH);
        let sample = sample_grayscale(
            &Image::load_from_444_png(Path::new(BALL_SAMPLE_PATH)).unwrap(),
            Circle {
                center: point![16.0, 16.0],
                radius: 16.0,
            },
        );
        let confidence = preclassify_sample(&mut network, &sample);

        println!("{confidence:?}");
        assert_relative_eq!(confidence, 1.0, epsilon = 0.01);
    }

    #[test]
    fn classify_ball() {
        let mut network = CompiledNN::default();
        network.compile(CLASSIFIER_PATH);
        let sample = sample_grayscale(
            &Image::load_from_444_png(Path::new(BALL_SAMPLE_PATH)).unwrap(),
            Circle {
                center: point![16.0, 16.0],
                radius: 16.0,
            },
        );
        let confidence = classify_sample(&mut network, &sample);
        let ball_confidence = confidence.ball;
        let feet_confidence = confidence.feet;
        let robot_part_confidence = confidence.robot_part;
        let penalty_spot_confidence = confidence.penalty_spot;

        println!("Ball: {ball_confidence:?}, Feet: {feet_confidence:?}, RobotPart: {robot_part_confidence:?}, PenaltySpot: {penalty_spot_confidence:?}");
        assert_relative_eq!(ball_confidence, 1.0, epsilon = 0.01);
    }

    #[test]
    fn position_ball() {
        let mut network = CompiledNN::default();
        network.compile(POSITIONER_PATH);
        let sample = sample_grayscale(
            &Image::load_from_444_png(Path::new(BALL_SAMPLE_PATH)).unwrap(),
            Circle {
                center: point![16.0, 16.0],
                radius: 16.0,
            },
        );
        let circle = position_sample(&mut network, &sample);

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
            grid_element: Circle {
                center: point![50.0, 50.0],
                radius: 32.0,
            },
            preclassifier_confidence: 1.0,
            classifier_confidence: Some(1.0),
            positioned_ball: Some(Circle {
                center: point![50.0, 50.0],
                radius: 32.0,
            }),
            positioned_feet: None,
            positioned_robot_part: None,
            positioned_penalty_spot: None,
            merge_weight: None,
        };
        let merge_weight =
            calculate_ball_merge_factor(&ball_candidate, vector!(90, 90), 1.0, 1.0, 1.0);
        assert_relative_eq!(merge_weight, 1.0);
    }

    #[test]
    fn candidate_evaluation_complex() {
        let ball_candidate = CandidateEvaluation {
            grid_element: Circle {
                center: point![50.0, 50.0],
                radius: 32.0,
            },
            preclassifier_confidence: 1.0,
            classifier_confidence: Some(0.5),
            positioned_ball: Some(Circle {
                center: point![66.0, 50.0],
                radius: 32.0,
            }),
            positioned_feet: None,
            positioned_robot_part: None,
            positioned_penalty_spot: None,
            merge_weight: None,
        };
        let merge_weight =
            calculate_ball_merge_factor(&ball_candidate, vector!(90, 90), 1.0, 1.0, 1.0);
        assert_relative_eq!(merge_weight, 0.5 * 0.75 * (7.0 / 8.0));
    }

    #[test]
    fn cycle_with_loaded_image() -> Result<()> {
        let filename = "../../tests/data/rome_bottom_ball.png";
        let image = Image::load_from_444_png(Path::new(filename))?;
        let configuration = BallDetectionConfiguration {
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
            object_candidates: AdditionalOutput::<Vec<CandidateEvaluation>>::new(
                false,
                &mut additional_output_buffer,
            ),
            configuration: &configuration,
            ball_radius: &0.5,
            camera_matrix: &camera_matrix,
            image: &image,
            perspective_grid_candidates: &perspective_grid_candidates,
        };
        let mut preclassifier = CompiledNN::default();
        preclassifier.compile(&context.configuration.preclassifier_neural_network);

        let mut classifier = CompiledNN::default();
        classifier.compile(&context.configuration.classifier_neural_network);

        let mut positioner = CompiledNN::default();
        positioner.compile(&context.configuration.positioner_neural_network);

        let neural_networks = NeuralNetworks {
            preclassifier,
            classifier,
            positioner,
        };
        let mut node = BallDetection { neural_networks };
        let balls = node.cycle(context)?.balls;
        assert!(balls.value.is_some());

        assert_eq!(balls.value.as_ref().unwrap().len(), 1);
        assert_relative_eq!(
            balls.value.unwrap()[0],
            Ball {
                position: point![0.374, 0.008],
                image_location: Circle {
                    center: point![308.93, 176.42],
                    radius: 42.92,
                }
            },
            epsilon = 0.01,
        );
        Ok(())
    }
}
