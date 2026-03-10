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
use ndarray::{ArrayView3, Axis, s};
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
    object_detection::{Detection, NaoLabelPartyObjectDetectionLabel},
    parameters::ObjectDetectionParameters,
};

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
    pub detected_objects: MainOutput<Vec<Detection<NaoLabelPartyObjectDetectionLabel>>>,
}

impl ObjectDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let tensor_rt = TensorRTExecutionProvider::default()
            .with_device_id(0)
            .with_fp16(true)
            .with_engine_cache(true)
            .with_engine_cache_path(paths.cache.join("tensor-rt").display())
            .build();
        let cuda = CUDAExecutionProvider::default().build();

        let session = Session::builder()?
            .with_execution_providers([tensor_rt, cuda])?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(2)?
            .commit_from_file(neural_network_folder.join("yolo26m-finetune-nv12.onnx"))?;

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

        if image.width % 64 != 0 || image.height % 64 != 0 {
            bail!(
                "image dimensions must be multiples of 64 (got {}x{})",
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
        let output = outputs["network_detections"]
            .try_extract_array::<f32>()?
            .t()
            .into_owned();

        let output = output.slice(s![.., .., 0]);

        let inference_duration = inference_start.elapsed();
        let post_processing_start = Instant::now();

        let mut candidate_detections: Vec<Detection<NaoLabelPartyObjectDetectionLabel>> = output
            .axis_iter(Axis(1))
            .filter_map(|row| {
                let confidence = row[4usize];
                let class_id = row[5usize] as usize;
                if confidence < context.parameters.confidence_threshold {
                    return None;
                }
                let label = NaoLabelPartyObjectDetectionLabel::from_index(class_id);
                Some(Detection {
                    bounding_box: BoundingBox {
                        area: Rectangle {
                            min: point!(row[0usize] * 2., row[1usize] * 2.),
                            max: point!(row[2usize] * 2., row[3usize] * 2.),
                        },
                        confidence,
                    },
                    label,
                })
            })
            .collect();

        candidate_detections.sort_by(|detection1, detection2| {
            detection1
                .bounding_box
                .confidence
                .total_cmp(&detection2.bounding_box.confidence)
        });

        let post_processing_duration = post_processing_start.elapsed();
        let non_maxiumum_suppression_start = Instant::now();

        let detected_objects = non_maximum_suppression(
            candidate_detections,
            context.parameters.maximum_intersection_over_union,
        );

        let non_maxiumum_suppression_duration = non_maxiumum_suppression_start.elapsed();

        context
            .inference_duration
            .fill_if_subscribed(|| inference_duration);

        context
            .post_processing_duration
            .fill_if_subscribed(|| post_processing_duration);

        context
            .non_maximum_suppression_duration
            .fill_if_subscribed(|| non_maxiumum_suppression_duration);

        Ok(MainOutputs {
            detected_objects: detected_objects.into(),
        })
    }
}

fn non_maximum_suppression<T>(
    mut sorted_candidate_detections: Vec<Detection<T>>,
    maximum_intersection_over_union: f32,
) -> Vec<Detection<T>> {
    let mut poses = Vec::new();

    while let Some(detection) = sorted_candidate_detections.pop() {
        sorted_candidate_detections.retain(|detection_candidate| {
            detection
                .bounding_box
                .intersection_over_union(&detection_candidate.bounding_box)
                < maximum_intersection_over_union
        });

        poses.push(detection)
    }

    poses
}
