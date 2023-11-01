use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use context_attribute::context;
use framework::{deserialize_not_implemented, MainOutput};
use hardware::PathsInterface;
use itertools::Itertools;
use nalgebra::point;
use ndarray::{Array2, ArrayView, ArrayView1};
use openvino::{Blob, Core, ExecutableNetwork, Layout, Precision, TensorDesc};
use serde::{Deserialize, Serialize};
use types::{
    color::{Rgb, YCbCr422, YCbCr444},
    geometry::Rectangle,
    object_detection::{BoundingBox, DetectedObject},
    ycbcr422_image::YCbCr422Image,
};

const IMAGE_DOWNSCALE_FACTOR: usize = 4;
const DETECTION_IMAGE_WIDTH: usize = 160;
const DETECTION_IMAGE_HEIGHT: usize = 120;
const DETECTION_NUMBER_CHANNELS: usize = 3;
const MAX_DETECTION: usize = 160;

const DETECTION_SCRATCHPAD_SIZE: usize =
    DETECTION_IMAGE_HEIGHT * DETECTION_IMAGE_WIDTH * DETECTION_NUMBER_CHANNELS;
type Scratchpad = [f32; DETECTION_SCRATCHPAD_SIZE];

#[derive(Deserialize, Serialize)]
pub struct SingleShotDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    scratchpad: Scratchpad,
    #[serde(skip, default = "deserialize_not_implemented")]
    network: ExecutableNetwork,

    input_name: String,
    output_name: String,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    image: Input<YCbCr422Image, "image">,

    iou_threshold: Parameter<f32, "detection.$cycler_instance.iou_threshold">,
    confidence_threshold: Parameter<f32, "detection.$cycler_instance.confidence_threshold">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    detections: MainOutput<Vec<BoundingBox>>,
}

impl SingleShotDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_path = dbg!(neural_network_folder.join("mobilenetv3_120_160_model-ov.xml"));
        let weights_path = dbg!(neural_network_folder.join("mobilenetv3_120_160_model-ov.bin"));

        let mut core = Core::new(None)?;
        let mut network = core
            .read_network_from_file(
                model_path
                    .to_str()
                    .wrap_err("failed to get detection model path")?,
                weights_path
                    .to_str()
                    .wrap_err("failed to get detection weights path")?,
            )
            .wrap_err("failed to create detection network")?;

        let input_name = network.get_input_name(0)?;
        let output_name = network.get_output_name(0)?;

        network
            .set_input_layout(&input_name, Layout::NCHW)
            .wrap_err("failed to set input data format")?;

        Ok(Self {
            scratchpad: [0.; DETECTION_SCRATCHPAD_SIZE],
            network: core.load_network(&network, "CPU")?,
            input_name,
            output_name,
        })
    }

    fn as_bytes(v: &[f32]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<f32>(),
            )
        }
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let image = context.image;
        SingleShotDetection::downsample_image_into_rgb2::<IMAGE_DOWNSCALE_FACTOR>(
            &mut self.scratchpad,
            image,
        );

        let mut infer_request = self.network.create_infer_request()?;

        let tensor_description = TensorDesc::new(Layout::NCHW, &[1, 3, 120, 160], Precision::FP32);
        let blob = Blob::new(
            &tensor_description,
            SingleShotDetection::as_bytes(&self.scratchpad[..]),
        )?;

        infer_request.set_blob(&self.input_name, &blob)?;
        infer_request.infer()?;

        let mut raw_boxes = infer_request.get_blob(&self.output_name)?;
        let raw_boxes = unsafe { raw_boxes.buffer_mut_as_type::<f32>().unwrap() };
        let boxes = ArrayView::from_shape((MAX_DETECTION, 4), raw_boxes)?;

        let mut raw_classification = infer_request.get_blob(&self.output_name)?;
        let raw_classification = unsafe { raw_classification.buffer_mut_as_type::<f32>().unwrap() };
        let classification = ArrayView::from_shape((MAX_DETECTION, 5), raw_classification)?;

        let bounding_boxes = classification
            .rows()
            .into_iter()
            .zip(boxes.rows())
            .filter_map(|(classification, bounding_box)| {
                retrieve_class(classification, *context.confidence_threshold).map(
                    |(score, detection)| {
                        BoundingBox::new(
                            detection,
                            score,
                            Rectangle {
                                min: point![bounding_box[0], bounding_box[1]],
                                max: point![bounding_box[2], bounding_box[3]],
                            },
                        )
                    },
                )
            })
            .collect_vec();

        let bounding_boxes = non_maximum_suppression(bounding_boxes, *context.iou_threshold);

        Ok(MainOutputs {
            detections: bounding_boxes.into(),
        })
    }

    pub fn downsample_image_into_rgb2<const DOWNSAMPLE_RATIO: usize>(
        scratchpad: &mut Scratchpad,
        image: &YCbCr422Image,
    ) {
        let width = image.width() as usize;
        let height = image.height() as usize;

        let downsampled_width = width / DOWNSAMPLE_RATIO;
        let downsampled_height = height / DOWNSAMPLE_RATIO;

        assert_eq!(downsampled_height, DETECTION_IMAGE_HEIGHT);
        assert_eq!(downsampled_width, DETECTION_IMAGE_WIDTH);

        let mut scratchpad_index = 0;
        const STRIDE: usize = DETECTION_IMAGE_HEIGHT * DETECTION_IMAGE_WIDTH;

        for row in image.buffer().chunks(width / 2).step_by(DOWNSAMPLE_RATIO) {
            for pixel in row.iter().step_by(DOWNSAMPLE_RATIO / 2) {
                let rgb = Rgb::from(*pixel);
                scratchpad[scratchpad_index + 0] = rgb.b as f32 / 255.0;
                scratchpad[scratchpad_index + STRIDE] = rgb.g as f32 / 255.0;
                scratchpad[scratchpad_index + 2 * STRIDE] = rgb.r as f32 / 255.0;
                scratchpad_index += 1;
            }
        }
        assert_eq!(scratchpad_index, STRIDE);
    }
}

fn retrieve_class<'a>(
    classification: ArrayView1<'a, f32>,
    confidence_threshold: f32,
) -> Option<(f32, DetectedObject)> {
    let total = classification.iter().map(|score| score.exp()).sum::<f32>();
    let highest_score_index = classification
        .iter()
        .enumerate()
        .max_by(|(_, &value0), (_, &value1)| value0.total_cmp(&value1))
        .map(|(idx, _)| idx as u8)
        .unwrap();

    let confidence = classification[highest_score_index as usize] / total;
    if confidence > confidence_threshold {
        DetectedObject::from_u8(highest_score_index).map(|object| (confidence, object))
    } else {
        None
    }
}

fn non_maximum_suppression(
    mut candidate_detections: Vec<BoundingBox>,
    iou_threshold: f32,
) -> Vec<BoundingBox> {
    let mut detections = Vec::new();
    candidate_detections.sort_unstable_by(|bbox1, bbox2| bbox1.score.total_cmp(&bbox2.score));

    while let Some(detection) = candidate_detections.pop() {
        let detection_area = detection.bounding_box.area();
        candidate_detections = candidate_detections
            .into_iter()
            .filter(|detection_candidate| {
                let intersection = detection
                    .bounding_box
                    .rectangle_intersection(detection_candidate.bounding_box);
                let candidate_area = detection_candidate.bounding_box.area();
                let iou = intersection / (detection_area + candidate_area - intersection);

                iou < iou_threshold
            })
            .collect_vec();

        detections.push(detection)
    }

    detections
}

#[cfg(test)]
mod tests {
    use ndarray::array;
    use types::{
        geometry::Rectangle,
        object_detection::{BoundingBox, DetectedObject},
    };

    use super::{retrieve_class, non_maximum_suppression};

    #[test]
    fn test_non_maximum_suppression() {
        let box1 = BoundingBox::new(
            DetectedObject::Robot,
            0.8,
            Rectangle {
                min: point![10.0, 10.0],
                max: point![100.0, 200.0],
            },
        );

        let box2 = BoundingBox::new(
            DetectedObject::Ball,
            0.2,
            Rectangle {
                min: point![20.0, 20.0],
                max: point![40.0, 40.0],
            },
        );

        let box3 = BoundingBox::new(
            DetectedObject::GoalPost,
            0.4,
            Rectangle {
                min: point![80.0, 20.0],
                max: point![121.0, 100.0],
            },
        );

        let results = non_maximum_suppression(vec![box1, box2, box3], 0.8);
        assert(results.len() == 1);
        assert(results[0] == box1);

        let results = non_maximum_suppression(vec![box1, box2, box3], 0.5);
        assert(results.len() == 2);
        assert(results[0] == box1);
        assert(results[0] == box3);
    }

    #[test]
    fn test_class_retrieval() {
        use DetectedObject::*;

        let background = array![1.0, 0.0, 0.0, 0.0, 0.0];
        assert!(matches!(retrieve_class(background.view(), 0.5), None));

        let robot = array![1.0, 2.0, 0.0, 0.0, 0.0];
        assert!(matches!(
            retrieve_class(robot.view(), 0.5),
            Some((_, Robot))
        ));

        let ball = array![0.1, 0.1, 10.0, 0.2, 0.0];
        assert!(matches!(retrieve_class(ball.view(), 0.5), Some((_, Ball))));

        let goal_post = array![0.0, 0.0, 0.0, 1.0, 0.0];
        assert!(matches!(
            retrieve_class(goal_post.view(), 0.5),
            Some((_, GoalPost))
        ));

        let penalty_spot = array![0.0, 0.0, 0.0, 0.0, 1.0];
        assert!(matches!(
            retrieve_class(penalty_spot.view(), 0.5),
            Some((_, GoalPost))
        ));

        let unsure_classification = array![1.0, 1.0, 1.0, 0.0, 0.0];
        assert!(matches!(
            retrieve_class(unsure_classification.view(), 0.5),
            None
        ));
    }
}
