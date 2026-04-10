use std::time::{Duration, Instant};

use color_eyre::{
    Result,
    eyre::{Context, bail},
};
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, deserialize_not_implemented};
use geometry::rectangle::Rectangle;
use hardware::PathsInterface;
use linear_algebra::point;
use ndarray::{Array2, ArrayView2, ArrayView3, ArrayViewD, Axis, Ix2, s};
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    inputs,
    session::{Session, SessionOutputs, builder::GraphOptimizationLevel},
    value::TensorRef,
};
use ros2::sensor_msgs::image::Image;
use serde::{Deserialize, Serialize};
use types::{
    bounding_box::BoundingBox,
    object_detection::{Object, RobocupObjectLabel, YOLOObjectLabel},
    parameters::ObjectDetectionParameters,
    pose_detection::{Keypoints, Pose},
};

const MODEL_FILE_NAME: &str = "hydra-nv12.onnx";
const DETECTION_OUTPUT_COLUMNS: usize = 6;
const POSE_OUTPUT_COLUMNS: usize = 57;
const POSE_KEYPOINT_OFFSET: usize = 6;

#[derive(Clone, Copy, Debug)]
enum TaskHead {
    ObjectDetection,
    PoseDetection,
}

impl TaskHead {
    fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::ObjectDetection => &[
                "network_detections_0",
                "detection_output",
                "network_detections",
            ],
            Self::PoseDetection => &["network_detections_1", "pose_output"],
        }
    }

    fn expected_columns(self) -> usize {
        match self {
            Self::ObjectDetection => DETECTION_OUTPUT_COLUMNS,
            Self::PoseDetection => POSE_OUTPUT_COLUMNS,
        }
    }

    fn output_name(self) -> &'static str {
        match self {
            Self::ObjectDetection => "object detection",
            Self::PoseDetection => "pose detection",
        }
    }
}

#[derive(Debug)]
struct ModelOutput {
    name: String,
    values: Array2<f32>,
}

#[derive(Deserialize, Serialize)]
pub struct ObjectDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    session: Session,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    image_left_raw: Input<Image, "image_left_raw">,

    inference_duration: AdditionalOutput<Duration, "inference_duration">,
    post_processing_duration: AdditionalOutput<Duration, "post_processing_duration">,
    non_maximum_suppression_duration:
        AdditionalOutput<Duration, "non_maximum_suppression_duration">,

    parameters: Parameter<ObjectDetectionParameters, "object_detection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_objects: MainOutput<Vec<Object<RobocupObjectLabel>>>,
    pub detected_poses: MainOutput<Vec<Pose<YOLOObjectLabel>>>,
}

impl ObjectDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let tensor_rt = TensorRTExecutionProvider::default()
            .with_device_id(0)
            .with_fp16(true)
            .with_engine_cache(true)
            .with_engine_cache_path(neural_network_folder.display())
            .build();
        let cuda = CUDAExecutionProvider::default().build();

        let session = Session::builder()?
            .with_execution_providers([tensor_rt, cuda])?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(2)?
            .commit_from_file(neural_network_folder.join(MODEL_FILE_NAME))?;

        Ok(Self { session })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if !context.parameters.enable {
            return Ok(MainOutputs::default());
        }
        let image = context.image_left_raw;
        if image.encoding != "nv12" {
            bail!("unsupported image encoding: {}", image.encoding);
        }

        if image.width % 32 != 0 || image.height % 32 != 0 {
            bail!(
                "image dimensions must be multiples of 32 (got {}x{})",
                image.width,
                image.height
            );
        }

        let nv12_data = ArrayView3::from_shape(
            [image.height as usize / 2, image.width as usize / 2, 6],
            image.data.as_slice(),
        )
        .wrap_err("failed to view nv12 data")?;

        let inference_start = Instant::now();
        let outputs: SessionOutputs = self
            .session
            .run(inputs!["raw_bytes_input" => TensorRef::from_array_view(nv12_data)?])?;

        let inference_duration = inference_start.elapsed();
        let post_processing_start = Instant::now();

        let model_outputs = extract_model_outputs(&outputs)?;
        let candidate_detections =
            extract_candidate_detections(&model_outputs, context.parameters.confidence_threshold)?;
        let candidate_human_poses =
            extract_candidate_human_poses(&model_outputs, context.parameters.confidence_threshold)?;

        let post_processing_duration = post_processing_start.elapsed();
        let non_maximum_suppression_start = Instant::now();

        let detected_objects = non_maximum_suppression(
            candidate_detections,
            context.parameters.maximum_intersection_over_union,
        );
        let detected_poses = non_maximum_suppression(
            candidate_human_poses,
            context.parameters.maximum_intersection_over_union,
        );

        let non_maximum_suppression_duration = non_maximum_suppression_start.elapsed();

        context
            .inference_duration
            .fill_if_subscribed(|| inference_duration);

        context
            .post_processing_duration
            .fill_if_subscribed(|| post_processing_duration);

        context
            .non_maximum_suppression_duration
            .fill_if_subscribed(|| non_maximum_suppression_duration);

        Ok(MainOutputs {
            detected_objects: detected_objects.into(),
            detected_poses: detected_poses.into(),
        })
    }
}

fn extract_model_outputs(outputs: &SessionOutputs) -> Result<Vec<ModelOutput>> {
    outputs
        .iter()
        .map(|(name, value)| {
            let tensor = value
                .try_extract_array::<f32>()
                .wrap_err_with(|| format!("failed to extract output `{name}` as tensor"))?;
            let values = squeeze_output_tensor(name, tensor)?;
            Ok(ModelOutput {
                name: name.to_string(),
                values,
            })
        })
        .collect()
}

fn squeeze_output_tensor(output_name: &str, tensor: ArrayViewD<'_, f32>) -> Result<Array2<f32>> {
    match tensor.ndim() {
        2 => tensor
            .into_dimensionality::<Ix2>()
            .map(|array| array.to_owned())
            .wrap_err_with(|| {
                format!("failed to interpret output `{output_name}` as rank-2 tensor")
            }),
        3 => {
            let shape = tensor.shape();
            if shape[0] == 1 {
                tensor
                    .index_axis(Axis(0), 0)
                    .into_dimensionality::<Ix2>()
                    .map(|array| array.to_owned())
                    .wrap_err_with(|| {
                        format!(
                            "failed to interpret output `{output_name}` batch axis as rank-2 tensor"
                        )
                    })
            } else if shape[2] == 1 {
                tensor
                    .index_axis(Axis(2), 0)
                    .into_dimensionality::<Ix2>()
                    .map(|array| array.reversed_axes().to_owned())
                    .wrap_err_with(|| {
                        format!(
                            "failed to interpret output `{output_name}` trailing axis as rank-2 tensor"
                        )
                    })
            } else {
                bail!(
                    "unsupported shape for output `{output_name}`: expected [1, rows, cols] or [cols, rows, 1], got {shape:?}"
                );
            }
        }
        rank => {
            bail!("unsupported rank for output `{output_name}`: expected rank 2 or 3, got {rank}")
        }
    }
}

fn select_task_output<'a>(
    task: TaskHead,
    outputs: &'a [ModelOutput],
) -> Result<ArrayView2<'a, f32>> {
    for alias in task.aliases() {
        if let Some(output) = outputs.iter().find(|output| output.name == *alias) {
            ensure_output_columns(task, output)?;
            return Ok(output.values.view());
        }
    }

    let matching_outputs = outputs
        .iter()
        .filter(|output| output.values.ncols() == task.expected_columns())
        .collect::<Vec<_>>();

    match matching_outputs.len() {
        1 => Ok(matching_outputs[0].values.view()),
        0 => bail!(
            "failed to locate {} output (aliases: {:?}); available outputs: {}",
            task.output_name(),
            task.aliases(),
            describe_outputs(&outputs.iter().collect::<Vec<_>>()),
        ),
        _ => bail!(
            "found multiple candidates for {} output with {} columns: {}",
            task.output_name(),
            task.expected_columns(),
            describe_outputs(&matching_outputs)
        ),
    }
}

fn ensure_output_columns(task: TaskHead, output: &ModelOutput) -> Result<()> {
    if output.values.ncols() < task.expected_columns() {
        bail!(
            "output `{}` has {} columns but {} expects at least {}",
            output.name,
            output.values.ncols(),
            task.output_name(),
            task.expected_columns(),
        );
    }
    Ok(())
}

fn describe_outputs(outputs: &[&ModelOutput]) -> String {
    outputs
        .iter()
        .map(|output| {
            format!(
                "{}({}x{})",
                output.name,
                output.values.nrows(),
                output.values.ncols()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn extract_candidate_detections(
    outputs: &[ModelOutput],
    confidence_threshold: f32,
) -> Result<Vec<Object<RobocupObjectLabel>>> {
    let output = select_task_output(TaskHead::ObjectDetection, outputs)?;

    Ok(output
        .axis_iter(Axis(0))
        .filter_map(|row| {
            let confidence = row[4usize];
            if confidence < confidence_threshold {
                return None;
            }

            let class_id = row[5usize] as usize;
            let label = RobocupObjectLabel::from_index(class_id);

            Some(Object {
                bounding_box: BoundingBox {
                    area: Rectangle {
                        min: point!(row[0usize], row[1usize]),
                        max: point!(row[2usize], row[3usize]),
                    },
                    confidence,
                },
                label,
            })
        })
        .collect())
}

fn extract_candidate_human_poses(
    outputs: &[ModelOutput],
    confidence_threshold: f32,
) -> Result<Vec<Pose<YOLOObjectLabel>>> {
    let output = select_task_output(TaskHead::PoseDetection, outputs)?;

    Ok(output
        .axis_iter(Axis(0))
        .filter_map(|row| {
            let confidence = row[4usize];
            if confidence < confidence_threshold {
                return None;
            }
            let label_index = row[5usize];
            let label = YOLOObjectLabel::from_index(label_index as usize);

            let keypoint_values = row.slice(s![POSE_KEYPOINT_OFFSET..]).to_vec();
            let keypoints = Keypoints::try_new(&keypoint_values, 0.0, 0.0)?;

            Some(Pose::new(
                Object {
                    label,
                    bounding_box: BoundingBox {
                        area: Rectangle {
                            min: point!(row[0usize], row[1usize]),
                            max: point!(row[2usize], row[3usize]),
                        },
                        confidence,
                    },
                },
                keypoints,
            ))
        })
        .collect())
}

trait HasBoundingBox {
    fn bounding_box(&self) -> &BoundingBox;
}

impl<T> HasBoundingBox for Object<T> {
    fn bounding_box(&self) -> &BoundingBox {
        &self.bounding_box
    }
}

impl<T> HasBoundingBox for Pose<T> {
    fn bounding_box(&self) -> &BoundingBox {
        &self.object.bounding_box
    }
}

fn non_maximum_suppression<T: HasBoundingBox>(
    mut sorted_candidate_detections: Vec<T>,
    maximum_intersection_over_union: f32,
) -> Vec<T> {
    sorted_candidate_detections.sort_by(|detection1, detection2| {
        detection1
            .bounding_box()
            .confidence
            .total_cmp(&detection2.bounding_box().confidence)
    });

    let mut remaining_detections = Vec::new();

    while let Some(detection) = sorted_candidate_detections.pop() {
        sorted_candidate_detections.retain(|detection_candidate| {
            detection
                .bounding_box()
                .intersection_over_union(detection_candidate.bounding_box())
                < maximum_intersection_over_union
        });

        remaining_detections.push(detection)
    }

    remaining_detections
}
