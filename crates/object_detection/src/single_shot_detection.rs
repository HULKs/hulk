use std::{time::Duration, path::PathBuf};

use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use context_attribute::context;
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use geometry::rectangle::Rectangle;
use hardware::{PathsInterface, TimeInterface};
use itertools::Itertools;
use ndarray::{s, ArrayView};
use openvino::{Blob, Core, ExecutableNetwork, Layout, Precision, TensorDesc};
use serde::{Deserialize, Serialize};
use types::{
    color::Rgb,
    object_detection::{BoundingBox, DetectedObject, Side},
    ycbcr422_image::YCbCr422Image,
};

const DETECTION_IMAGE_SIZE: usize = 256;
const DETECTION_NUMBER_CHANNELS: usize = 3;

const MAX_DETECTION: usize = 1344;

const DETECTION_SCRATCHPAD_SIZE: usize = DETECTION_IMAGE_SIZE.pow(2) * DETECTION_NUMBER_CHANNELS;
type Scratchpad = [f32; DETECTION_SCRATCHPAD_SIZE];

#[derive(Deserialize, Serialize)]
pub struct SingleShotDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    scratchpad: Scratchpad,
    #[serde(skip, default = "deserialize_not_implemented")]
    network: ExecutableNetwork,

    input_name: String,
    output_name: String,

    last_side: Side,
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

        let model_xml_name = PathBuf::from("yolov8n-mobilenetv3-160-224.xml");

        let model_path = neural_network_folder.join(&model_xml_name);
        let weights_path = neural_network_folder.join(model_xml_name.with_extension("bin"));

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
            last_side: Side::Left,
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

        self.last_side = self.last_side.opposite();

        let image = context.image;
        let earlier = context.hardware_interface.get_now();

        SingleShotDetection::load_into_scratchpad(&mut self.scratchpad, image, self.last_side);

        context.preprocess_time.fill_if_subscribed(|| {
            context
                .hardware_interface
                .get_now()
                .duration_since(earlier)
                .expect("time ran backwards")
        });

        let mut infer_request = self.network.create_infer_request()?;

        let tensor_description = TensorDesc::new(
            Layout::NCHW,
            &[
                1,
                DETECTION_NUMBER_CHANNELS,
                DETECTION_IMAGE_SIZE,
                DETECTION_IMAGE_SIZE,
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
            context
                .hardware_interface
                .get_now()
                .duration_since(earlier)
                .expect("time ran backwards")
        });
        let mut prediction = infer_request.get_blob("output0")?;
        let prediction = unsafe { prediction.buffer_mut_as_type::<f32>().unwrap() };
        let prediction = ArrayView::from_shape((8, MAX_DETECTION), prediction)?;

        let earlier = context.hardware_interface.get_now();
        let detections = prediction
            .columns()
            .into_iter()
            .filter_map(|row| {
                let (class_id, prob) = row
                    .slice(s![4..])
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (index, *value))
                    .reduce(|accum, row| if row.1 > accum.1 { row } else { accum })
                    .unwrap();

                if prob < *context.confidence_threshold {
                    return None;
                }
                let object = DetectedObject::from_u8(class_id as u8 + 1).unwrap();
                let bbox = row.slice(s![0..4]);

                let right_shift = if let Side::Left = self.last_side {
                    0.
                } else {
                    320. - 256.
                };

                Some(BoundingBox::new(
                    object,
                    prob,
                    Rectangle::from_cxcywh(
                        (bbox[0] + right_shift) * 2.0,
                        bbox[1] * 2.0,
                        bbox[2] * 2.0,
                        bbox[3] * 2.0,
                    ),
                ))
            })
            .collect_vec();

        let bounding_boxes = multiclass_non_maximum_suppression(detections, *context.iou_threshold);

        context.postprocess_time.fill_if_subscribed(|| {
            context
                .hardware_interface
                .get_now()
                .duration_since(earlier)
                .expect("time ran backwards")
        });

        Ok(MainOutputs {
            detections: bounding_boxes.into(),
        })
    }

    pub fn load_into_scratchpad(scratchpad: &mut Scratchpad, image: &YCbCr422Image, side: Side) {
        let height = image.height();
        let width = image.width();

        const DOWNSAMPLE_RATIO: usize = 2;
        const STRIDE: usize = DETECTION_IMAGE_SIZE.pow(2);
        let mut scratchpad_index = 0;

        if let Side::Left = side {
            for y in (0..height).step_by(DOWNSAMPLE_RATIO) {
                for x in (0..(DOWNSAMPLE_RATIO * DETECTION_IMAGE_SIZE)).step_by(DOWNSAMPLE_RATIO) {
                    let pixel: Rgb = image.at(x as u32, y as u32).into();

                    scratchpad[scratchpad_index + 0 * STRIDE] = pixel.r as f32 / 255.;
                    scratchpad[scratchpad_index + 1 * STRIDE] = pixel.g as f32 / 255.;
                    scratchpad[scratchpad_index + 2 * STRIDE] = pixel.b as f32 / 255.;

                    scratchpad_index += 1;
                }
            }
        } else {
            for y in (0..height).step_by(DOWNSAMPLE_RATIO) {
                for x in
                    ((width - 2 * DETECTION_IMAGE_SIZE as u32)..width).step_by(DOWNSAMPLE_RATIO)
                {
                    let pixel: Rgb = image.at(x, y).into();

                    scratchpad[scratchpad_index + 0 * STRIDE] = pixel.r as f32 / 255.;
                    scratchpad[scratchpad_index + 1 * STRIDE] = pixel.g as f32 / 255.;
                    scratchpad[scratchpad_index + 2 * STRIDE] = pixel.b as f32 / 255.;

                    scratchpad_index += 1;
                }
            }
        }
    }
}

// fn retrieve_class<'a>(
//     classification: ArrayView1<'a, f32>,
//     confidence_threshold: f32,
// ) -> Option<(f32, DetectedObject)> {
//     let total = classification.iter().map(|score| score.exp()).sum::<f32>();
//     let highest_score_index = classification
//         .iter()
//         .enumerate()
//         .max_by(|(_, &value0), (_, &value1)| value0.total_cmp(&value1))
//         .map(|(idx, _)| idx as u8)
//         .unwrap();

//     let confidence = classification[highest_score_index as usize].exp() / total;
//     if confidence > confidence_threshold {
//         DetectedObject::from_u8(highest_score_index).map(|object| (confidence, object))
//     } else {
//         None
//     }
// }

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

// #[cfg(test)]
// mod tests {
//     use approx::assert_relative_eq;
//     use geometry::rectangle::Rectangle;
//     use nalgebra::point;
//     use ndarray::array;
//     use types::object_detection::{BoundingBox, DetectedObject};

//     use super::{multiclass_non_maximum_suppression, retrieve_class};

//     fn assert_approx_bbox_equality(bbox1: BoundingBox, bbox2: BoundingBox) {
//         assert_relative_eq!(bbox1.score, bbox2.score);
//         assert_relative_eq!(bbox1.bounding_box.min, bbox2.bounding_box.min);
//         assert_relative_eq!(bbox1.bounding_box.max, bbox2.bounding_box.max);
//         assert_eq!(bbox1.class, bbox2.class);
//     }

//     #[test]
//     fn test_non_maximum_suppression() {
//         let box1 = BoundingBox::new(
//             DetectedObject::Robot,
//             0.8,
//             Rectangle {
//                 min: point![10.0, 10.0],
//                 max: point![100.0, 200.0],
//             },
//         );

//         let box2 = BoundingBox::new(
//             DetectedObject::Ball,
//             0.2,
//             Rectangle {
//                 min: point![20.0, 20.0],
//                 max: point![40.0, 40.0],
//             },
//         );

//         let box3 = BoundingBox::new(
//             DetectedObject::Robot,
//             0.4,
//             Rectangle {
//                 min: point![10.0, 10.0],
//                 max: point![190.0, 200.0],
//             },
//         );

//         let results = multiclass_non_maximum_suppression(vec![box1, box2, box3], 0.6);
//         assert!(results.len() == 3);
//         assert_approx_bbox_equality(results[0], box1);
//         assert_approx_bbox_equality(results[1], box3);
//         assert_approx_bbox_equality(results[2], box2);

//         let results = multiclass_non_maximum_suppression(vec![box1, box2, box3], 0.45);
//         assert!(results.len() == 2);
//         assert_approx_bbox_equality(results[0], box1);
//         assert_approx_bbox_equality(results[1], box2);
//     }

//     #[test]
//     fn test_class_retrieval() {
//         use DetectedObject::*;

//         let background = array![1.0, 0.0, 0.0, 0.0, 0.0];
//         assert!(matches!(retrieve_class(background.view(), 0.5), None));

//         let robot = array![1.0, 2.0, 0.0, 0.0, 0.0];
//         assert!(matches!(
//             retrieve_class(robot.view(), 0.5),
//             Some((_, Robot))
//         ));

//         let ball = array![0.1, 0.1, 10.0, 0.2, 0.0];
//         assert!(matches!(retrieve_class(ball.view(), 0.5), Some((_, Ball))));

//         let goal_post = array![0.0, 0.0, 0.0, 1.0, 0.0];
//         assert!(matches!(
//             retrieve_class(goal_post.view(), 0.4),
//             Some((_, GoalPost))
//         ));

//         let penalty_spot = array![0.0, 0.0, 0.0, 0.0, 1.0];
//         assert!(matches!(
//             retrieve_class(penalty_spot.view(), 0.4),
//             Some((_, PenaltySpot))
//         ));

//         let unsure_classification = array![1.0, 1.0, 1.0, 0.0, 0.0];
//         assert!(matches!(
//             retrieve_class(unsure_classification.view(), 0.3),
//             None
//         ));
//     }
// }
