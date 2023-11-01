use std::time::Duration;

use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use context_attribute::context;
use framework::{deserialize_not_implemented, MainOutput, AdditionalOutput};
use geometry::rectangle::Rectangle;
use hardware::{PathsInterface, TimeInterface};
use itertools::Itertools;
use nalgebra::point;
use ndarray::{ArrayView, ArrayView1};
use openvino::{Blob, Core, ExecutableNetwork, Layout, Precision, TensorDesc};
use serde::{Deserialize, Serialize};
use types::{
    color::Rgb,
    object_detection::{BoundingBox, DetectedObject},
    ycbcr422_image::YCbCr422Image,
};

const DETECTION_IMAGE_WIDTH: usize = 320;
const DETECTION_IMAGE_HEIGHT: usize = 240;
const DETECTION_NUMBER_CHANNELS: usize = 3;

const MAX_DETECTION: usize = 1440;

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
    preprocess_time: AdditionalOutput<Duration, "preprocess_time">,
    inference_time: AdditionalOutput<Duration, "inference_time">,
    postprocess_time: AdditionalOutput<Duration, "postprocess_time">,

    image: Input<YCbCr422Image, "image">,
    hardware_interface: HardwareInterface,

    iou_threshold: Parameter<f32, "detection.$cycler_instance.iou_threshold">,
    confidence_threshold: Parameter<f32, "detection.$cycler_instance.confidence_threshold">,
    minimum_area: Parameter<f32, "detection.$cycler_instance.minimum_area">,
    enable: Parameter<bool, "detection.$cycler_instance.enable">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detections: MainOutput<Vec<BoundingBox>>,
}

impl SingleShotDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_path =
            dbg!(neural_network_folder.join("simplified-mobilenetv3_240_320_model-ov.xml"));
        let weights_path =
            dbg!(neural_network_folder.join("simplified-mobilenetv3_240_320_model-ov.bin"));

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

    pub fn cycle(&mut self, mut context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        if !context.enable {
            return Ok(MainOutputs::default());
        }

        let image = context.image;
        const DOWNSCALE_FACTOR_WIDTH: usize = 640 / DETECTION_IMAGE_WIDTH;
        const DOWNSCALE_FACTOR_HEIGHT: usize = 480 / DETECTION_IMAGE_HEIGHT;
        assert_eq!(
            DOWNSCALE_FACTOR_HEIGHT, DOWNSCALE_FACTOR_WIDTH,
            "the downscaling needs to be equal in both directions"
        );
        
        let earlier = context.hardware_interface.get_now();
        SingleShotDetection::downsample_image_into_rgb2::<DOWNSCALE_FACTOR_HEIGHT>(
            &mut self.scratchpad,
            image,
        );
        context.preprocess_time.fill_if_subscribed(|| {
            context.hardware_interface.get_now().duration_since(earlier).expect("time ran backwards")
        });

        let mut infer_request = self.network.create_infer_request()?;

        let tensor_description = TensorDesc::new(
            Layout::NCHW,
            &[
                1,
                DETECTION_NUMBER_CHANNELS,
                DETECTION_IMAGE_HEIGHT,
                DETECTION_IMAGE_WIDTH,
            ],
            Precision::FP32,
        );
        let blob = Blob::new(
            &tensor_description,
            SingleShotDetection::as_bytes(&self.scratchpad[..]),
        )?;

        
        let earlier = context.hardware_interface.get_now();
        infer_request.set_blob(&self.input_name, &blob)?;
        infer_request.infer()?;
        context.inference_time.fill_if_subscribed(|| {
            context.hardware_interface.get_now().duration_since(earlier).expect("time ran backwards")
        });

        let mut raw_boxes = infer_request.get_blob("boxes")?;
        let raw_boxes = unsafe { raw_boxes.buffer_mut_as_type::<f32>().unwrap() };
        let boxes = ArrayView::from_shape((MAX_DETECTION, 4), raw_boxes)?;

        let mut raw_classification = infer_request.get_blob("scores")?;
        let raw_classification = unsafe { raw_classification.buffer_mut_as_type::<f32>().unwrap() };
        let classification = ArrayView::from_shape((MAX_DETECTION, 5), raw_classification)?;

        let width_scale = 640. / DETECTION_IMAGE_WIDTH as f32;
        let height_scale = 480. / DETECTION_IMAGE_HEIGHT as f32;
        
        let earlier = context.hardware_interface.get_now();
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
                                min: point![
                                    bounding_box[0] * width_scale,
                                    bounding_box[1] * height_scale
                                ],
                                max: point![
                                    bounding_box[2] * width_scale,
                                    bounding_box[3] * height_scale
                                ],
                            },
                        )
                    },
                )
            })
            .collect_vec();

        let bounding_boxes = bounding_boxes
            .into_iter()
            .filter(|detection| detection.bounding_box.area() > *context.minimum_area)
            .collect_vec();
        let bounding_boxes =
            multiclass_non_maximum_suppression(bounding_boxes, *context.iou_threshold);

        context.postprocess_time.fill_if_subscribed(|| {
            context.hardware_interface.get_now().duration_since(earlier).expect("time ran backwards")
        });

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

    let confidence = classification[highest_score_index as usize].exp() / total;
    if confidence > confidence_threshold {
        DetectedObject::from_u8(highest_score_index).map(|object| (confidence, object))
    } else {
        None
    }
}

fn multiclass_non_maximum_suppression(
    mut candidate_detections: Vec<BoundingBox>,
    iou_threshold: f32,
) -> Vec<BoundingBox> {
    let mut detections = Vec::new();
    candidate_detections.sort_unstable_by(|bbox1, bbox2| bbox1.score.total_cmp(&bbox2.score));

    while let Some(detection) = candidate_detections.pop() {
        candidate_detections = candidate_detections
            .into_iter()
            .filter(|detection_candidate| {
                detection.class != detection_candidate.class
                    || detection.iou(detection_candidate) < iou_threshold
            })
            .collect_vec();

        detections.push(detection)
    }

    detections
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use geometry::rectangle::Rectangle;
    use nalgebra::point;
    use ndarray::array;
    use types::object_detection::{BoundingBox, DetectedObject};

    use super::{multiclass_non_maximum_suppression, retrieve_class};

    fn assert_approx_bbox_equality(bbox1: BoundingBox, bbox2: BoundingBox) {
        assert_relative_eq!(bbox1.score, bbox2.score);
        assert_relative_eq!(bbox1.bounding_box.min, bbox2.bounding_box.min);
        assert_relative_eq!(bbox1.bounding_box.max, bbox2.bounding_box.max);
        assert_eq!(bbox1.class, bbox2.class);
    }

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
            DetectedObject::Robot,
            0.4,
            Rectangle {
                min: point![10.0, 10.0],
                max: point![190.0, 200.0],
            },
        );

        let results = multiclass_non_maximum_suppression(vec![box1, box2, box3], 0.6);
        assert!(results.len() == 3);
        assert_approx_bbox_equality(results[0], box1);
        assert_approx_bbox_equality(results[1], box3);
        assert_approx_bbox_equality(results[2], box2);

        let results = multiclass_non_maximum_suppression(vec![box1, box2, box3], 0.45);
        assert!(results.len() == 2);
        assert_approx_bbox_equality(results[0], box1);
        assert_approx_bbox_equality(results[1], box2);
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
            retrieve_class(goal_post.view(), 0.4),
            Some((_, GoalPost))
        ));

        let penalty_spot = array![0.0, 0.0, 0.0, 0.0, 1.0];
        assert!(matches!(
            retrieve_class(penalty_spot.view(), 0.4),
            Some((_, PenaltySpot))
        ));

        let unsure_classification = array![1.0, 1.0, 1.0, 0.0, 0.0];
        assert!(matches!(
            retrieve_class(unsure_classification.view(), 0.3),
            None
        ));
    }
}
