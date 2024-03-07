use std::{path::PathBuf, time::Duration};

use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use context_attribute::context;
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use geometry::rectangle::Rectangle;
use hardware::{PathsInterface, TimeInterface};
use itertools::Itertools;
use lazy_static::lazy_static;
use ndarray::{s, ArrayView};
use openvino::{Blob, Core, ExecutableNetwork, InferRequest, Layout, Precision, TensorDesc};
use serde::{Deserialize, Serialize};
use types::{color::Rgb, object_detection::DetectedRobot, ycbcr422_image::YCbCr422Image};

const DETECTION_IMAGE_HEIGHT: usize = 96;
const DETECTION_IMAGE_WIDTH: usize = 128;
const DETECTION_NUMBER_CHANNELS: usize = 3;
const NUMBER_OF_CLASSES: usize = 1;

const MAX_DETECTION: usize = 252;

const DETECTION_SCRATCHPAD_SIZE: usize =
    DETECTION_IMAGE_WIDTH * DETECTION_IMAGE_HEIGHT * DETECTION_NUMBER_CHANNELS;

lazy_static! {
    pub static ref X_INDICES: Vec<u32> = compute_indices(DETECTION_IMAGE_WIDTH, 640);
    pub static ref Y_INDICES: Vec<u32> = compute_indices(DETECTION_IMAGE_HEIGHT, 480);
}

fn compute_indices(detection_size: usize, image_size: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity(detection_size);
    let stride = image_size as f32 / detection_size as f32;
    for i in 0..detection_size {
        indices.push((i as f32 * stride).round() as u32);
    }
    indices
}

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
    enable: Parameter<bool, "detection.$cycler_instance.enable">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detections: MainOutput<Option<Vec<DetectedRobot>>>,
}

impl SingleShotDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_xml_name = PathBuf::from("yolov8n-mobilenet.xml");

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
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        if !context.enable {
            return Ok(MainOutputs::default());
        }
        let image = context.image;

        {
            let earlier = context.hardware_interface.get_now();
            SingleShotDetection::load_into_scratchpad(&mut self.scratchpad, image);
            context.preprocess_time.fill_if_subscribed(|| {
                context
                    .hardware_interface
                    .get_now()
                    .duration_since(earlier)
                    .expect("time ran backwards")
            });
        }

        let mut prediction = {
            let mut infer_request = self.set_network_inputs()?;

            let earlier = context.hardware_interface.get_now();
            infer_request.infer()?;

            context.inference_time.fill_if_subscribed(|| {
                context
                    .hardware_interface
                    .get_now()
                    .duration_since(earlier)
                    .expect("time ran backwards")
            });

            infer_request.get_blob(&self.output_name)?
        };

        let prediction = unsafe { prediction.buffer_mut_as_type::<f32>().unwrap() };

        let earlier = context.hardware_interface.get_now();
        let detections = self.parse_outputs(prediction, *context.confidence_threshold)?;

        let bounding_boxes = non_maximum_suppression(detections, *context.iou_threshold);

        context.postprocess_time.fill_if_subscribed(|| {
            context
                .hardware_interface
                .get_now()
                .duration_since(earlier)
                .expect("time ran backwards")
        });

        Ok(MainOutputs {
            detections: Some(bounding_boxes).into(),
        })
    }

    fn set_network_inputs(&mut self) -> Result<InferRequest> {
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
        let blob = Blob::new(&tensor_description, as_bytes(&self.scratchpad[..]))?;

        infer_request.set_blob(&self.input_name, &blob)?;

        Ok(infer_request)
    }

    fn parse_outputs(
        &self,
        prediction: &[f32],
        confidence_threshold: f32,
    ) -> Result<Vec<DetectedRobot>> {
        let prediction = ArrayView::from_shape((4 + NUMBER_OF_CLASSES, MAX_DETECTION), prediction)?;

        let detections = prediction
            .columns()
            .into_iter()
            .filter_map(|row| {
                let confidence = row[4];

                if confidence < confidence_threshold {
                    return None;
                }
                let bbox = row.slice(s![0..4]);

                const X_SCALE: f32 = 640.0 / DETECTION_IMAGE_WIDTH as f32;
                const Y_SCALE: f32 = 480.0 / DETECTION_IMAGE_HEIGHT as f32;

                Some(DetectedRobot::new(
                    confidence,
                    Rectangle::from_cxcywh(
                        bbox[0] * X_SCALE,
                        bbox[1] * Y_SCALE,
                        bbox[2] * X_SCALE,
                        bbox[3] * Y_SCALE,
                    ),
                ))
            })
            .collect();

        Ok(detections)
    }

    fn load_into_scratchpad(scratchpad: &mut Scratchpad, image: &YCbCr422Image) {
        const STRIDE: usize = DETECTION_IMAGE_HEIGHT * DETECTION_IMAGE_WIDTH;

        let mut scratchpad_index = 0;
        for &y in Y_INDICES.iter() {
            for &x in X_INDICES.iter() {
                let pixel: Rgb = image.at(x, y).into();

                scratchpad[scratchpad_index] = pixel.r as f32 / 255.;
                scratchpad[scratchpad_index + STRIDE] = pixel.g as f32 / 255.;
                scratchpad[scratchpad_index + 2 * STRIDE] = pixel.b as f32 / 255.;

                scratchpad_index += 1;
            }
        }
    }
}

fn as_bytes(float_slice: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            float_slice.as_ptr() as *const u8,
            std::mem::size_of_val(float_slice),
        )
    }
}

fn non_maximum_suppression(
    mut candidate_detections: Vec<DetectedRobot>,
    iou_threshold: f32,
) -> Vec<DetectedRobot> {
    let mut detections = Vec::new();
    candidate_detections.sort_unstable_by(|bbox1, bbox2| bbox1.score.total_cmp(&bbox2.score));

    while let Some(detection) = candidate_detections.pop() {
        candidate_detections = candidate_detections
            .into_iter()
            .filter(|detection_candidate| detection.iou(detection_candidate) < iou_threshold)
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
    use types::object_detection::DetectedRobot;

    use super::non_maximum_suppression;

    const BOX_1: DetectedRobot = DetectedRobot::new(
        0.8,
        Rectangle {
            min: point![10.0, 10.0],
            max: point![100.0, 200.0],
        },
    );

    const BOX_2: DetectedRobot = DetectedRobot::new(
        0.2,
        Rectangle {
            min: point![200.0, 200.0],
            max: point![300.0, 400.0],
        },
    );

    const BOX_3: DetectedRobot = DetectedRobot::new(
        0.4,
        Rectangle {
            min: point![11.0, 11.0],
            max: point![99.0, 199.0],
        },
    );

    fn assert_approx_bbox_equality(bbox1: DetectedRobot, bbox2: DetectedRobot) {
        assert_relative_eq!(bbox1.score, bbox2.score);
        assert_relative_eq!(bbox1.bounding_box.min, bbox2.bounding_box.min);
        assert_relative_eq!(bbox1.bounding_box.max, bbox2.bounding_box.max);
    }

    #[test]
    fn test_bbox_equality() {
        assert_approx_bbox_equality(BOX_1, BOX_1);
        assert_approx_bbox_equality(BOX_2, BOX_2);
        assert_approx_bbox_equality(BOX_3, BOX_3);
    }

    #[test]
    fn test_non_maximum_suppression_for_single_box() {
        let results = non_maximum_suppression(vec![BOX_1], 0.6);
        assert!(results.len() == 1);
        assert_approx_bbox_equality(results[0], BOX_1);
    }

    #[test]
    fn test_non_maximum_suppression_non_overlapping_boxes() {
        let results = non_maximum_suppression(vec![BOX_1, BOX_2], 0.6);
        assert!(results.len() == 2);

        assert_approx_bbox_equality(results[0], BOX_1);
        assert_approx_bbox_equality(results[1], BOX_2);
    }

    #[test]
    fn test_non_maximum_suppression_overlapping_boxes() {
        let results = non_maximum_suppression(vec![BOX_1, BOX_3], 0.6);
        assert!(results.len() == 1);
        assert_approx_bbox_equality(results[0], BOX_1);
    }

    #[test]
    fn test_non_maximum_suppression_overlapping_boxes_with_stricter_threshold() {
        let results = non_maximum_suppression(vec![BOX_1, BOX_3], 1.0);
        assert!(results.len() == 2);
        assert_approx_bbox_equality(results[0], BOX_1);
        assert_approx_bbox_equality(results[1], BOX_3);
    }

    #[test]
    fn test_non_maximum_suppression_all_boxes() {
        let results = non_maximum_suppression(vec![BOX_1, BOX_2, BOX_3], 0.6);
        assert!(results.len() == 2);
        assert_approx_bbox_equality(results[0], BOX_1);
        assert_approx_bbox_equality(results[1], BOX_2);
    }
}
