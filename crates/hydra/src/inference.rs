use std::time::{Duration, Instant};

use color_eyre::{
    Result,
    eyre::{Context, Ok, bail},
};
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, deserialize_not_implemented};
use hardware::PathsInterface;
use ndarray::{ArrayView2, ArrayView3, Axis, Ix3};
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
    object_detection::{NUMBER_OF_VALUES_PER_OBJECT, Object, RobocupObjectLabel, YOLOObjectLabel},
    parameters::HydraParameters,
    pose_detection::{NUMBER_OF_VALUES_PER_POSE, Pose},
    segmentation_detection::{
        NUMBER_OF_VALUES_PER_SEGMENTED_OBJECT, PROTOTYPE_MASK_CHANNELS, PROTOTYPE_MASK_HEIGHT,
        PROTOTYPE_MASK_WIDTH, SegmentedObject,
    },
};

const MODEL_FILE_NAME: &str =
    "yolo26m=f11+yolo26m~ruckus+yolo26m-pose~saloon+yolo26m-seg~promise.onnx";
pub const NUMBER_OF_DETECTIONS: usize = 300;

#[derive(Clone, Copy, Debug)]
enum TaskOutput {
    ObjectDetection,
    PoseDetection,
    SegmentationObjects,
    SegmentationPrototypes,
}

impl TaskOutput {
    fn output_name(self) -> &'static str {
        match self {
            TaskOutput::ObjectDetection => "object_output",
            TaskOutput::PoseDetection => "pose_output",
            TaskOutput::SegmentationObjects => "segmentation_output",
            TaskOutput::SegmentationPrototypes => "segmentation_proto",
        }
    }

    fn expected_shape(self) -> &'static [usize] {
        match self {
            Self::ObjectDetection => &[1, NUMBER_OF_DETECTIONS, NUMBER_OF_VALUES_PER_OBJECT],
            Self::PoseDetection => &[1, NUMBER_OF_DETECTIONS, NUMBER_OF_VALUES_PER_POSE],
            Self::SegmentationObjects => &[
                1,
                NUMBER_OF_DETECTIONS,
                NUMBER_OF_VALUES_PER_SEGMENTED_OBJECT,
            ],
            Self::SegmentationPrototypes => &[
                1,
                PROTOTYPE_MASK_CHANNELS,
                PROTOTYPE_MASK_HEIGHT,
                PROTOTYPE_MASK_WIDTH,
            ],
        }
    }
}

#[derive(Debug)]
struct ModelOutputs<'a> {
    objects: ArrayView2<'a, f32>,
    poses: ArrayView2<'a, f32>,
    segmentation_objects: ArrayView2<'a, f32>,
    segmentation_prototypes: ArrayView3<'a, f32>,
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

    parameters: Parameter<HydraParameters, "hydra">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_objects: MainOutput<Vec<Object<RobocupObjectLabel>>>,
    pub detected_poses: MainOutput<Vec<Pose<YOLOObjectLabel>>>,
    pub detected_segments: MainOutput<Vec<SegmentedObject<YOLOObjectLabel>>>,
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

        if !image.width.is_multiple_of(32) || !image.height.is_multiple_of(32) {
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

        let outputs = extract_outputs(&outputs)?;
        let candidate_detections = extract_candidate_object_detections(
            &outputs,
            context
                .parameters
                .object_detection_parameters
                .confidence_threshold,
        )?;
        let candidate_human_poses = extract_candidate_pose_detections(
            &outputs,
            context
                .parameters
                .pose_detection_parameters
                .confidence_threshold,
        )?;
        let candidate_segments = extract_candidate_segmentation_objects(
            &outputs,
            context
                .parameters
                .segmentation_detection_parameters
                .confidence_threshold,
        )?;

        let post_processing_duration = post_processing_start.elapsed();
        let non_maximum_suppression_start = Instant::now();

        let detected_objects = non_maximum_suppression(
            candidate_detections,
            context
                .parameters
                .object_detection_parameters
                .maximum_intersection_over_union,
        );
        let detected_poses = non_maximum_suppression(
            candidate_human_poses,
            context
                .parameters
                .pose_detection_parameters
                .maximum_intersection_over_union,
        );
        let detected_segments = non_maximum_suppression(
            candidate_segments,
            context
                .parameters
                .segmentation_detection_parameters
                .maximum_intersection_over_union,
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
            detected_segments: detected_segments.into(),
        })
    }
}

fn extract_outputs<'a>(outputs: &'a SessionOutputs<'a>) -> Result<ModelOutputs<'a>> {
    let objects_output =
        outputs[TaskOutput::ObjectDetection.output_name()].try_extract_array::<f32>()?;
    if objects_output.shape() != TaskOutput::ObjectDetection.expected_shape() {
        bail!(
            "object detection output not of expected shape. Expected: {:?}, got: {:?}",
            TaskOutput::ObjectDetection.expected_shape(),
            objects_output.shape()
        )
    }
    let reshaped_objects_output = objects_output.squeeze().into_dimensionality()?;

    let poses_output =
        outputs[TaskOutput::PoseDetection.output_name()].try_extract_array::<f32>()?;
    if poses_output.shape() != TaskOutput::PoseDetection.expected_shape() {
        bail!(
            "pose detection output not of expected shape. Expected: {:?}, got: {:?}",
            TaskOutput::PoseDetection.expected_shape(),
            poses_output.shape()
        )
    }
    let reshaped_pose_output = poses_output.squeeze().into_dimensionality()?;

    let segmentation_objects_output =
        outputs[TaskOutput::SegmentationObjects.output_name()].try_extract_array::<f32>()?;
    if segmentation_objects_output.shape() != TaskOutput::SegmentationObjects.expected_shape() {
        bail!(
            "segmentation objects output not of expected shape. Expected: {:?}, got: {:?}",
            TaskOutput::SegmentationObjects.expected_shape(),
            segmentation_objects_output.shape()
        )
    }
    let reshaped_segmentation_objects = segmentation_objects_output
        .squeeze()
        .into_dimensionality()?;

    let segmentation_proto_output =
        outputs[TaskOutput::SegmentationPrototypes.output_name()].try_extract_array::<f32>()?;
    if segmentation_proto_output.shape() != TaskOutput::SegmentationPrototypes.expected_shape() {
        bail!(
            "segmentation prototypes output not of expected shape. Expected: {:?}, got: {:?}",
            TaskOutput::SegmentationPrototypes.expected_shape(),
            segmentation_proto_output.shape()
        )
    }
    let reshaped_segmentation_prototypes = segmentation_proto_output
        .squeeze()
        .into_dimensionality::<Ix3>()?;

    Ok(ModelOutputs {
        objects: reshaped_objects_output,
        poses: reshaped_pose_output,
        segmentation_objects: reshaped_segmentation_objects,
        segmentation_prototypes: reshaped_segmentation_prototypes,
    })
}

fn extract_candidate_object_detections(
    outputs: &ModelOutputs,
    confidence_threshold: f32,
) -> Result<Vec<Object<RobocupObjectLabel>>> {
    Ok(outputs
        .objects
        .axis_iter(Axis(0))
        .filter_map(|row| {
            let confidence = row[4usize];
            if confidence < confidence_threshold {
                return None;
            }

            let object_values: [f32; NUMBER_OF_VALUES_PER_OBJECT] = row
                .as_slice()
                .expect("slice is not contiguous")
                .try_into()
                .unwrap_or_else(|_| {
                    panic!("slice is not of length {}", NUMBER_OF_VALUES_PER_OBJECT)
                });

            Some(Object::from(object_values))
        })
        .collect())
}

fn extract_candidate_pose_detections(
    outputs: &ModelOutputs,
    confidence_threshold: f32,
) -> Result<Vec<Pose<YOLOObjectLabel>>> {
    Ok(outputs
        .poses
        .axis_iter(Axis(0))
        .filter_map(|row| {
            let confidence = row[4usize];
            if confidence < confidence_threshold {
                return None;
            }

            let pose_values: [f32; NUMBER_OF_VALUES_PER_POSE] = row
                .as_slice()
                .expect("slice is not contiguous")
                .try_into()
                .unwrap_or_else(|_| panic!("slice is not of length {}", NUMBER_OF_VALUES_PER_POSE));

            Some(Pose::from(&pose_values))
        })
        .collect())
}

fn extract_candidate_segmentation_objects(
    outputs: &ModelOutputs,
    confidence_threshold: f32,
) -> Result<Vec<SegmentedObject<YOLOObjectLabel>>> {
    Ok(outputs
        .segmentation_objects
        .axis_iter(Axis(0))
        .filter_map(|row| {
            let confidence = row[4usize];
            if confidence < confidence_threshold {
                return None;
            }

            let seg_values: [f32; NUMBER_OF_VALUES_PER_SEGMENTED_OBJECT] = row
                .as_slice()
                .expect("slice is not contiguous")
                .try_into()
                .unwrap_or_else(|_| {
                    panic!(
                        "slice is not of length {}",
                        NUMBER_OF_VALUES_PER_SEGMENTED_OBJECT
                    )
                });

            Some(SegmentedObject::from((
                &seg_values,
                outputs.segmentation_prototypes,
            )))
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

impl<T> HasBoundingBox for SegmentedObject<T> {
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
