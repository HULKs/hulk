use color_eyre::{
    eyre::{eyre, Ok},
    Result,
};
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
    session::{Session, SessionOutputs},
    value::TensorRef,
};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use serde::{Deserialize, Serialize};
use types::{bounding_box::BoundingBox, object_detection::Detection};

#[rustfmt::skip]
const YOLOV8_CLASS_LABELS: [&str; 80] = [
    "person", "bicycle", "car", "motorcycle", "airplane", "bus", "train", "truck", "boat", "traffic light",
	"fire hydrant", "stop sign", "parking meter", "bench", "bird", "cat", "dog", "horse", "sheep", "cow", "elephant",
	"bear", "zebra", "giraffe", "backpack", "umbrella", "handbag", "tie", "suitcase", "frisbee", "skis", "snowboard",
	"sports ball", "kite", "baseball bat", "baseball glove", "skateboard", "surfboard", "tennis racket", "bottle",
	"wine glass", "cup", "fork", "knife", "spoon", "bowl", "banana", "apple", "sandwich", "orange", "broccoli",
	"carrot", "hot dog", "pizza", "donut", "cake", "chair", "couch", "potted plant", "bed", "dining table", "toilet",
	"tv", "laptop", "mouse", "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink", "refrigerator",
	"book", "clock", "vase", "scissors", "teddy bear", "hair drier", "toothbrush"
];

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
    rgb_image: Input<Image, "rgb_image">,
    rgb_image_camera_info: Input<CameraInfo, "rgb_image_camera_info">,

    maximum_intersection_over_union:
        Parameter<f32, "object_detection.maximum_intersection_over_union">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub object_detections: MainOutput<Vec<Detection>>,
}

impl ObjectDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let session = Session::builder()?
            .with_execution_providers([
                TensorRTExecutionProvider::default().build(),
                CUDAExecutionProvider::default().build(),
            ])?
            .commit_from_file(neural_network_folder.join("yolo12n.onnx"))?;

        Ok(Self { session })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let height = context.rgb_image_camera_info.height;
        let width = context.rgb_image_camera_info.width;
        let rgb_image = if context.rgb_image.encoding == "rgb8" {
            RgbImage::from_raw(width, height, context.rgb_image.data.clone()).unwrap()
        } else {
            return Err(eyre!("unsupported image encoding"));
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

        // Run YOLOv8 inference
        let outputs: SessionOutputs = self
            .session
            .run(inputs!["images" => TensorRef::from_array_view(&input)?])?;
        let output = outputs["output0"]
            .try_extract_array::<f32>()?
            .t()
            .into_owned();

        let output = output.slice(s![.., .., 0]);
        let mut candidate_detection: Vec<Detection> = output
            .axis_iter(Axis(0))
            .filter_map(|row| {
                let row: Vec<_> = row.iter().copied().collect();
                let (class_id, prob) = row
                    .iter()
                    // skip bounding box coordinates
                    .skip(4)
                    .enumerate()
                    .map(|(index, value)| (index, *value))
                    .reduce(|accum, row| if row.1 > accum.1 { row } else { accum })
                    .unwrap();
                if prob >= 0.5 {
                    let label = YOLOV8_CLASS_LABELS[class_id];
                    let xc = row[0] / width as f32 * (width as f32);
                    let yc = row[1] / height as f32 * (height as f32);
                    let w = row[2] / width as f32 * (width as f32);
                    let h = row[3] / height as f32 * (height as f32);
                    Some(Detection {
                        bounding_box: BoundingBox {
                            area: Rectangle::new_with_center_and_size(
                                point!(xc, yc),
                                vector!(w, h),
                            ),
                            confidence: prob,
                        },
                        label: label.to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        candidate_detection.sort_by(|detection1, detection2| {
            detection1
                .bounding_box
                .confidence
                .total_cmp(&detection2.bounding_box.confidence)
        });

        let detections = non_maximum_suppression(
            candidate_detection,
            *context.maximum_intersection_over_union,
        );

        Ok(MainOutputs {
            object_detections: detections.into(),
        })
    }
}

fn non_maximum_suppression(
    mut candidate_detection: Vec<Detection>,
    maximum_intersection_over_union: f32,
) -> Vec<Detection> {
    let mut poses = Vec::new();
    candidate_detection.sort_unstable_by(|pose1, pose2| {
        pose1
            .bounding_box
            .confidence
            .total_cmp(&pose2.bounding_box.confidence)
    });

    while let Some(detection) = candidate_detection.pop() {
        candidate_detection.retain(|detection_candidate| {
            detection
                .bounding_box
                .intersection_over_union(&detection_candidate.bounding_box)
                < maximum_intersection_over_union
        });

        poses.push(detection)
    }

    poses
}
