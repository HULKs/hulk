use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use context_attribute::context;
use coordinate_systems::Pixel;
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use geometry::rectangle::Rectangle;
use hardware::{PathsInterface, TimeInterface};
use itertools::Itertools;
use lazy_static::lazy_static;
use linear_algebra::{point, vector};
use ndarray::{s, ArrayView};
use openvino::{Blob, Core, ExecutableNetwork, Layout, Precision, TensorDesc};
use serde::{Deserialize, Serialize};
use types::{
    bounding_box::BoundingBox,
    color::Rgb,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    pose_detection::{HumanPose, Keypoints},
    ycbcr422_image::YCbCr422Image,
};

const DETECTION_IMAGE_HEIGHT: usize = 256;
const DETECTION_IMAGE_WIDTH: usize = 320;
const DETECTION_NUMBER_CHANNELS: usize = 3;

const MAX_DETECTION: usize = 1680;

const DETECTION_SCRATCHPAD_SIZE: usize =
    DETECTION_IMAGE_WIDTH * DETECTION_IMAGE_HEIGHT * DETECTION_NUMBER_CHANNELS;

const X_SCALE: f32 = 640.0 / DETECTION_IMAGE_WIDTH as f32;
const Y_SCALE: f32 = 480.0 / DETECTION_IMAGE_HEIGHT as f32;

const STRIDE: usize = DETECTION_IMAGE_HEIGHT * DETECTION_IMAGE_WIDTH;

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
pub struct PoseDetection {
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
    hardware_interface: HardwareInterface,

    preprocess_time: AdditionalOutput<Duration, "preprocess_time">,
    inference_time: AdditionalOutput<Duration, "inference_time">,
    postprocess_time: AdditionalOutput<Duration, "postprocess_time">,

    image: Input<YCbCr422Image, "image">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "Control", "filtered_game_controller_state?">,

    intersection_over_union_threshold:
        Parameter<f32, "detection.$cycler_instance.intersection_over_union_threshold">,
    keypoint_confidence_threshold:
        Parameter<f32, "detection.$cycler_instance.keypoint_confidence_threshold">,
    enable: Parameter<bool, "detection.$cycler_instance.enable">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub human_poses: MainOutput<Option<Vec<HumanPose>>>,
}

impl PoseDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_xml_name = PathBuf::from("yolov8n-pose-ov.xml");

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

    fn as_bytes(v: &[f32]) -> &[u8] {
        unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
    }

    pub fn cycle(&mut self, mut context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        if !context.enable {
            return Ok(MainOutputs::default());
        }

        if let Some(filtered_game_controller_state) = context.filtered_game_controller_state {
            if filtered_game_controller_state.game_state != FilteredGameState::Initial {
                return Ok(MainOutputs::default());
            }
        }

        let image = context.image;
        let earlier = context.hardware_interface.get_now();

        load_into_scratchpad(&mut self.scratchpad, image);

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
                DETECTION_IMAGE_HEIGHT,
                DETECTION_IMAGE_WIDTH,
            ],
            Precision::FP32,
        );
        let blob = Blob::new(
            &tensor_description,
            PoseDetection::as_bytes(&self.scratchpad[..]),
        )?;

        let earlier = SystemTime::now();

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
        let prediction = ArrayView::from_shape((56, MAX_DETECTION), prediction)?;

        let earlier = SystemTime::now();
        let poses = prediction
            .columns()
            .into_iter()
            .filter_map(|row| {
                let prob = row[4];
                if prob < *context.keypoint_confidence_threshold {
                    return None;
                }
                let bounding_box_slice = row.slice(s![0..4]);

                // bbox re-scale
                let center_x = bounding_box_slice[0] * X_SCALE;
                let center_y = bounding_box_slice[1] * Y_SCALE;
                let center = point![center_x, center_y];

                let width = bounding_box_slice[2] * X_SCALE;
                let height = bounding_box_slice[3] * Y_SCALE;
                let size = vector![width, height];

                let bounding_box = BoundingBox::new(
                    prob,
                    Rectangle::<Pixel>::new_with_center_and_size(center, size),
                );

                let keypoints_slice = row.slice(s![5..]);
                let keypoints = Keypoints::try_new(
                    keypoints_slice.as_standard_layout().as_slice()?,
                    X_SCALE,
                    Y_SCALE,
                )?;
                Some(HumanPose::new(bounding_box, keypoints))
            })
            .collect_vec();

        let poses = non_maximum_suppression(poses, *context.intersection_over_union_threshold);

        context.postprocess_time.fill_if_subscribed(|| {
            SystemTime::now()
                .duration_since(earlier)
                .expect("time ran backwards")
        });

        Ok(MainOutputs {
            human_poses: Some(poses).into(),
        })
    }
}

fn load_into_scratchpad(scratchpad: &mut Scratchpad, image: &YCbCr422Image) {
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

fn non_maximum_suppression(
    mut candiate_pose: Vec<HumanPose>,
    intersection_over_union_threshold: f32,
) -> Vec<HumanPose> {
    let mut poses = Vec::new();
    candiate_pose.sort_unstable_by(|pose1, pose2| {
        pose1
            .bounding_box
            .score
            .total_cmp(&pose2.bounding_box.score)
    });

    while let Some(detection) = candiate_pose.pop() {
        candiate_pose = candiate_pose
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
