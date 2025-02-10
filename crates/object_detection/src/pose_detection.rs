use std::time::{Duration, SystemTime};

use color_eyre::{
    eyre::{bail, eyre, Context, ContextCompat},
    Result,
};
use itertools::Itertools;
use ndarray::{s, ArrayView};
use openvino::{
    CompiledModel, Core, DeviceType, ElementType, InferenceError::GeneralError, Tensor,
};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Pixel;
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use geometry::rectangle::Rectangle;
use hardware::PathsInterface;
use linear_algebra::{point, vector};
use types::{
    bounding_box::BoundingBox,
    color::Rgb,
    motion_command::MotionCommand,
    pose_detection::{HumanPose, Keypoints},
    ycbcr422_image::YCbCr422Image,
};

const DETECTION_IMAGE_HEIGHT: usize = 480;
const DETECTION_IMAGE_WIDTH: usize = 192;
const DETECTION_IMAGE_START_X: usize = (640 - DETECTION_IMAGE_WIDTH) / 2;

const EXPECTED_OUTPUT_NAME: &str = "detections";

const MAX_DETECTIONS: usize = 1890;

const STRIDE: usize = DETECTION_IMAGE_HEIGHT * DETECTION_IMAGE_WIDTH;

#[derive(Deserialize, Serialize)]
pub struct PoseDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    network: CompiledModel,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    preprocess_duration: AdditionalOutput<Duration, "preprocess_duration">,
    inference_duration: AdditionalOutput<Duration, "inference_duration">,
    postprocess_duration: AdditionalOutput<Duration, "postprocess_duration">,

    image: Input<YCbCr422Image, "image">,
    motion_command: Input<MotionCommand, "Control", "motion_command">,

    maximum_intersection_over_union:
        Parameter<f32, "pose_detection.maximum_intersection_over_union">,
    minimum_bounding_box_confidence:
        Parameter<f32, "pose_detection.minimum_bounding_box_confidence">,
    override_pose_detection: Parameter<bool, "pose_detection.override_pose_detection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub unfiltered_human_poses: MainOutput<Vec<HumanPose>>,
}

impl PoseDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_xml_name = "yolo11n-pose-ov.xml";

        let model_path = neural_network_folder.join(model_xml_name);
        let weights_path = neural_network_folder
            .join(model_xml_name)
            .with_extension("bin");

        let mut core = Core::new()?;
        let network = core
            .read_model_from_file(
                model_path
                    .to_str()
                    .wrap_err("failed to get detection model path")?,
                weights_path
                    .to_str()
                    .wrap_err("failed to get detection weights path")?,
            )
            .map_err(|error| match error {
                GeneralError => eyre!("{error}: possible incomplete OpenVino installation"),
                _ => eyre!("{error}: failed to create detection network"),
            })?;

        let number_of_inputs = network
            .get_inputs_len()
            .wrap_err("failed to get number of inputs")?;
        let output_name = network.get_output_by_index(0)?.get_name()?;
        if number_of_inputs != 1 || output_name != EXPECTED_OUTPUT_NAME {
            bail!(
                "expected exactly one input and output name to be '{}'",
                EXPECTED_OUTPUT_NAME
            );
        }

        Ok(Self {
            network: core.compile_model(&network, DeviceType::CPU)?,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let behavior_requests_pose_detection = matches!(
            context.motion_command,
            MotionCommand::Initial {
                should_look_for_referee: true,
                ..
            } | MotionCommand::Stand {
                should_look_for_referee: true,
                ..
            }
        );
        if !behavior_requests_pose_detection && !context.override_pose_detection {
            return Ok(MainOutputs::default());
        };

        let image = context.image;

        let mut tensor = Tensor::new(ElementType::F32, &self.network.get_input()?.get_shape()?)?;
        {
            let earlier = SystemTime::now();

            load_into_scratchpad(tensor.get_data_mut()?, image);

            context.preprocess_duration.fill_if_subscribed(|| {
                SystemTime::now()
                    .duration_since(earlier)
                    .expect("time ran backwards")
            });
        }

        let mut infer_request = self.network.create_infer_request()?;

        infer_request.set_input_tensor(&tensor)?;

        {
            let earlier = SystemTime::now();

            infer_request.infer()?;
            context.inference_duration.fill_if_subscribed(|| {
                SystemTime::now()
                    .duration_since(earlier)
                    .expect("time ran backwards")
            });
        }
        let prediction = infer_request.get_output_tensor_by_index(0)?;
        let prediction =
            ArrayView::from_shape((56, MAX_DETECTIONS), prediction.get_data::<f32>()?)?;

        let earlier = SystemTime::now();
        let poses = prediction
            .columns()
            .into_iter()
            .filter_map(|row| {
                let confidence = row[4];
                if confidence < *context.minimum_bounding_box_confidence {
                    return None;
                }
                let bounding_box_slice = row.slice(s![0..4]);

                // bbox re-scale
                let center_x = bounding_box_slice[0] + DETECTION_IMAGE_START_X as f32;
                let center_y = bounding_box_slice[1];
                let center = point![center_x, center_y];

                let width = bounding_box_slice[2];
                let height = bounding_box_slice[3];
                let size = vector![width, height];

                let bounding_box = BoundingBox {
                    area: Rectangle::<Pixel>::new_with_center_and_size(center, size),
                    confidence,
                };

                let keypoints_slice = row.slice(s![5..]);
                let keypoints = Keypoints::try_new(
                    keypoints_slice.as_standard_layout().as_slice()?,
                    DETECTION_IMAGE_START_X as f32,
                    0.0,
                )?;
                Some(HumanPose::new(bounding_box, keypoints))
            })
            .collect_vec();

        let poses = non_maximum_suppression(poses, *context.maximum_intersection_over_union);

        context.postprocess_duration.fill_if_subscribed(|| {
            SystemTime::now()
                .duration_since(earlier)
                .expect("time ran backwards")
        });

        Ok(MainOutputs {
            unfiltered_human_poses: poses.into(),
        })
    }
}

fn load_into_scratchpad(scratchpad: &mut [f32], image: &YCbCr422Image) {
    let mut scratchpad_index = 0;
    for y in 0..DETECTION_IMAGE_HEIGHT as u32 {
        for x in
            DETECTION_IMAGE_START_X as u32..(DETECTION_IMAGE_START_X + DETECTION_IMAGE_WIDTH) as u32
        {
            let pixel: Rgb = image.at(x, y).into();

            scratchpad[scratchpad_index] = pixel.red as f32 / 255.;
            scratchpad[scratchpad_index + STRIDE] = pixel.green as f32 / 255.;
            scratchpad[scratchpad_index + 2 * STRIDE] = pixel.blue as f32 / 255.;

            scratchpad_index += 1;
        }
    }
}

fn non_maximum_suppression(
    mut candidate_pose: Vec<HumanPose>,
    maximum_intersection_over_union: f32,
) -> Vec<HumanPose> {
    let mut poses = Vec::new();
    candidate_pose.sort_unstable_by(|pose1, pose2| {
        pose1
            .bounding_box
            .confidence
            .total_cmp(&pose2.bounding_box.confidence)
    });

    while let Some(detection) = candidate_pose.pop() {
        candidate_pose = candidate_pose
            .into_iter()
            .filter(|detection_candidate| {
                detection
                    .bounding_box
                    .intersection_over_union(&detection_candidate.bounding_box)
                    < maximum_intersection_over_union
            })
            .collect_vec();

        poses.push(detection)
    }

    poses
}
