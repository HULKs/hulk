use std::time::{Duration, Instant};

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, deserialize_not_implemented};
use geometry::rectangle::Rectangle;
use hardware::PathsInterface;
use image::RgbImage;
use linear_algebra::point;
use ndarray::{Array, Axis, s};
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
    using_subsampled_image: bool,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    image_left_raw: Input<Image, "image_left_raw">,

    pre_processing_duration: AdditionalOutput<Duration, "preprocessing_duration">,
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
            .with_intra_threads(1)?
            .commit_from_file(neural_network_folder.join("yolo26m-finetune-640x544.onnx"))?;

        Ok(Self {
            session,
            using_subsampled_image: true,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if !context.parameters.enable {
            return Ok(MainOutputs::default());
        }

        let image_conversion_start = Instant::now();

        let mut image = context.image_left_raw.clone();

        if (image.height, image.width) == (1088, 1280) && image.encoding == "nv12" {
            log::info!("sub sampling image by half");
            image.subsample_nv12_by_half_in_place()?;
            self.using_subsampled_image = true;
        };

        let height = image.height;
        let width = image.width;
        assert_eq!((height, width), (544, 640));

        let Ok(rgb_image): Result<RgbImage, _> = image.try_into() else {
            return Ok(MainOutputs::default());
        };

        let mut input = Array::zeros((1, 3, height as usize, width as usize));
        for (x, y, pixel) in rgb_image.enumerate_pixels() {
            let x = x as _;
            let y = y as _;
            let [r, g, b] = pixel.0;
            input[[0, 0, y, x]] = (r as f32) / 255.;
            input[[0, 1, y, x]] = (g as f32) / 255.;
            input[[0, 2, y, x]] = (b as f32) / 255.;
        }

        let pre_processing_duration = image_conversion_start.elapsed();
        let inference_start = Instant::now();

        let outputs: SessionOutputs = self
            .session
            .run(inputs!["images" => TensorRef::from_array_view(&input)?])?;
        let output = outputs["output0"]
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
                    bounding_box: if self.using_subsampled_image {
                        BoundingBox {
                            area: Rectangle {
                                min: point!(row[0usize] * 2.0, row[1usize] * 2.0),
                                max: point!(row[2usize] * 2.0, row[3usize] * 2.0),
                            },
                            confidence,
                        }
                    } else {
                        BoundingBox {
                            area: Rectangle {
                                min: point!(row[0usize], row[1usize]),
                                max: point!(row[2usize], row[3usize]),
                            },
                            confidence,
                        }
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
            .pre_processing_duration
            .fill_if_subscribed(|| pre_processing_duration);

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
