use color_eyre::Result;
use context_attribute::context;
use framework::{deserialize_not_implemented, MainOutput};
use geometry::rectangle::Rectangle;
use hardware::PathsInterface;
use image::RgbImage;
use linear_algebra::{point, vector};
use ndarray::{s, Array, Axis};
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    inputs,
    session::{builder::GraphOptimizationLevel, Session, SessionOutputs},
    value::TensorRef,
};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use serde::{Deserialize, Serialize};
use types::{
    bounding_box::BoundingBox,
    object_detection::{Detection, YOLOv8ObjectDetectionLabel},
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
    image_left_raw_camera_info: Input<CameraInfo, "image_left_raw_camera_info">,

    parameters: Parameter<ObjectDetectionParameters, "object_detection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_objects: MainOutput<Vec<Detection>>,
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
            .build()
            .error_on_failure();
        let cuda = CUDAExecutionProvider::default().build();

        let session = Session::builder()?
            .with_execution_providers([tensor_rt, cuda])?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(1)?
            .commit_from_file(neural_network_folder.join("yolo11n-544x448.onnx"))?;

        Ok(Self { session })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if !context.parameters.enable {
            return Ok(MainOutputs::default());
        }

        let height = context.image_left_raw_camera_info.height;
        let width = context.image_left_raw_camera_info.width;

        let Ok(rgb_image): Result<RgbImage, _> = context.image_left_raw.clone().try_into() else {
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

        let outputs: SessionOutputs = self
            .session
            .run(inputs!["images" => TensorRef::from_array_view(&input)?])?;
        let output = outputs["output0"]
            .try_extract_array::<f32>()?
            .t()
            .into_owned();

        let output = output.slice(s![.., .., 0]);
        let mut candidate_detections: Vec<Detection> = output
            .axis_iter(Axis(0))
            .filter_map(|row| {
                let (class_id, confidence) = row
                    .iter()
                    .skip(4)
                    // skip bounding box coordinates
                    .enumerate()
                    .max_by(|(_, value_x), (_, value_y)| value_x.total_cmp(value_y))
                    .unwrap();
                if *confidence < context.parameters.confidence_threshold {
                    return None;
                }
                let label = YOLOv8ObjectDetectionLabel::from_index(class_id);
                Some(Detection {
                    bounding_box: BoundingBox {
                        area: Rectangle::new_with_center_and_size(
                            point!(row[0usize], row[1usize]),
                            vector!(row[2usize], row[3usize]),
                        ),
                        confidence: *confidence,
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

        let detected_objects = non_maximum_suppression(
            candidate_detections,
            context.parameters.maximum_intersection_over_union,
        );

        Ok(MainOutputs {
            detected_objects: detected_objects.into(),
        })
    }
}

fn non_maximum_suppression(
    mut sorted_candidate_detections: Vec<Detection>,
    maximum_intersection_over_union: f32,
) -> Vec<Detection> {
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
