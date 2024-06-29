use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use color_eyre::{
    eyre::{bail, eyre, Context, ContextCompat},
    Result,
};
use context_attribute::context;
use coordinate_systems::Pixel;
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use geometry::rectangle::Rectangle;
use hardware::{PathsInterface, TimeInterface};
use itertools::Itertools;
use linear_algebra::{point, vector};
use ndarray::{s, ArrayView};
use openvino::{
    CompiledModel, Core, DeviceType, ElementType, InferenceError::GeneralError, Tensor,
};
use serde::{Deserialize, Serialize};
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

const MAX_DETECTIONS: usize = 1890;

const STRIDE: usize = DETECTION_IMAGE_HEIGHT * DETECTION_IMAGE_WIDTH;

#[derive(Deserialize, Serialize)]
pub struct PoseDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    scratchpad: Box<[f32]>,
    #[serde(skip, default = "deserialize_not_implemented")]
    network: CompiledModel,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,

    preprocess_duration: AdditionalOutput<Duration, "preprocess_duration">,
    inference_duration: AdditionalOutput<Duration, "inference_duration">,
    postprocess_duration: AdditionalOutput<Duration, "postprocess_duration">,

    image: Input<YCbCr422Image, "image">,
    motion_command: Input<MotionCommand, "Control", "motion_command">,

    intersection_over_union_threshold:
        Parameter<f32, "object_detection.$cycler_instance.intersection_over_union_threshold">,
    keypoint_confidence_threshold:
        Parameter<f32, "object_detection.$cycler_instance.keypoint_confidence_threshold">,
    enable: Parameter<bool, "object_detection.$cycler_instance.enable">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub human_poses: MainOutput<Vec<HumanPose>>,
}

impl PoseDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_xml_name = PathBuf::from("yolov8n-pose-ov.xml");

        let model_path = neural_network_folder.join(&model_xml_name);
        let weights_path = neural_network_folder.join(model_xml_name.with_extension("bin"));

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
        let number_of_outputs = network
            .get_outputs_len()
            .wrap_err("failed to get number of outputs")?;

        if number_of_inputs != 1 || number_of_outputs != 1 {
            bail!("expected exactly one input and one output");
        }

        let input_shape = network
            .get_input_by_index(0)
            .wrap_err("failed to get input node")?
            .get_shape()
            .wrap_err("failed to get shape of input node")?;
        let number_of_elements = input_shape.get_dimensions().iter().product::<i64>();
        let scratchpad = vec![0.0; number_of_elements as usize].into_boxed_slice();

        Ok(Self {
            scratchpad,
            network: core.compile_model(&network, DeviceType::CPU)?,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        if !context.enable {
            return Ok(MainOutputs::default());
        }

        let should_look_for_referee = matches!(
            context.motion_command,
            MotionCommand::Initial {
                should_look_for_referee: true,
                ..
            }
        );
        if !should_look_for_referee {
            return Ok(MainOutputs::default());
        };

        let image = context.image;
        {
            let earlier = SystemTime::now();

            load_into_scratchpad(self.scratchpad.as_mut(), image);

            context.preprocess_duration.fill_if_subscribed(|| {
                SystemTime::now()
                    .duration_since(earlier)
                    .expect("time ran backwards")
            });
        }

        let mut infer_request = self.network.create_infer_request()?;
        let mut tensor = Tensor::new(ElementType::F32, &self.network.get_input()?.get_shape()?)?;

        tensor
            .get_raw_data_mut()?
            .copy_from_slice(self.scratchpad.as_bytes());

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
        let prediction = infer_request.get_output_tensor()?;
        let prediction =
            ArrayView::from_shape((56, MAX_DETECTIONS), prediction.get_data::<f32>()?)?;

        let earlier = SystemTime::now();
        let poses = prediction
            .columns()
            .into_iter()
            .filter_map(|row| {
                let confidence = row[4];
                if confidence < *context.keypoint_confidence_threshold {
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
                    score: confidence,
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

        let poses = non_maximum_suppression(poses, *context.intersection_over_union_threshold);

        context.postprocess_duration.fill_if_subscribed(|| {
            SystemTime::now()
                .duration_since(earlier)
                .expect("time ran backwards")
        });

        Ok(MainOutputs {
            human_poses: poses.into(),
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

            scratchpad[scratchpad_index] = pixel.r as f32 / 255.;
            scratchpad[scratchpad_index + STRIDE] = pixel.g as f32 / 255.;
            scratchpad[scratchpad_index + 2 * STRIDE] = pixel.b as f32 / 255.;

            scratchpad_index += 1;
        }
    }
}

fn non_maximum_suppression(
    mut candidate_pose: Vec<HumanPose>,
    intersection_over_union_threshold: f32,
) -> Vec<HumanPose> {
    let mut poses = Vec::new();
    candidate_pose.sort_unstable_by(|pose1, pose2| {
        pose1
            .bounding_box
            .score
            .total_cmp(&pose2.bounding_box.score)
    });

    while let Some(detection) = candidate_pose.pop() {
        candidate_pose = candidate_pose
            .into_iter()
            .filter(|detection_candidate| {
                detection
                    .bounding_box
                    .intersection_over_union(&detection_candidate.bounding_box)
                    < intersection_over_union_threshold
            })
            .collect_vec();

        poses.push(detection)
    }

    poses
}

trait AsBytes {
    fn as_bytes(&self) -> &[u8];
}

impl AsBytes for [f32] {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.as_ptr() as *const u8, std::mem::size_of_val(self))
        }
    }
}
