use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use hardware::TimeInterface;
use itertools::Itertools;
use nalgebra::center;
use nalgebra::distance;
use nalgebra::Isometry2;
use nalgebra::Point2;
use projection::Projection;
use serde::{Deserialize, Serialize};
use types::camera_matrix::CameraMatrices;
use types::camera_matrix::CameraMatrix;
use types::field_dimensions::FieldDimensions;
use types::{pose_detection::HumanPose, pose_types::PoseType, ycbcr422_image::YCbCr422Image};

#[derive(Deserialize, Serialize)]
pub struct PoseInterpretation {
    interpreted_pose_type: PoseType,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,

    camera_matrices: RequiredInput<Option<CameraMatrices>, "Control", "camera_matrices?">,
    human_poses: Input<Vec<HumanPose>, "human_poses">,
    image: Input<YCbCr422Image, "image">,
    robot_to_field: Input<Option<Isometry2<f32>>, "Control", "robot_to_field?">,
    robot_to_field_of_home_after_coin_toss_before_second_half: Input<
        Option<Isometry2<f32>>,
        "Control",
        "robot_to_field_of_home_after_coin_toss_before_second_half?",
    >,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    distance_to_center_threshhold:
        Parameter<f32, "detection.$cycler_instance.distance_to_center_threshhold">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub interpreted_pose_type: MainOutput<Option<PoseType>>,
}

impl PoseInterpretation {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        Ok(PoseInterpretation {
            interpreted_pose_type: PoseType::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        let referee_pose = Self::get_referee_pose(
            context.human_poses.clone(),
            context.camera_matrices.top.clone(),
            *context.robot_to_field.unwrap(),
            (context
                .robot_to_field_of_home_after_coin_toss_before_second_half
                .unwrap()
                * Point2::new(0.0, 0.0))
            .into(),
            *context.distance_to_center_threshhold,
        );
        let pose_type = Self::interpret_pose(&referee_pose);

        Ok(MainOutputs {
            interpreted_pose_type: Some(pose_type).into(),
        })
    }

    pub fn get_referee_pose(
        poses: Vec<HumanPose>,
        camera_matrix_top: CameraMatrix,
        robot_to_field: Isometry2<f32>,
        expected_referee_position: Point2<f32>,
        distance_to_center_threshhold: f32,
    ) -> HumanPose {
        let referee_pose_candidates: Vec<(&HumanPose, f32)> = poses
            .iter()
            .filter_map(|pose| {
                let left_foot_field_position: Point2<f32> = robot_to_field
                    * &camera_matrix_top
                        .pixel_to_ground(pose.keypoints.left_foot.point)
                        .unwrap();
                let right_foot_field_position: Point2<f32> = robot_to_field
                    * &camera_matrix_top
                        .pixel_to_ground(pose.keypoints.left_foot.point)
                        .unwrap();
                let distance_to_center = distance(
                    &center(&left_foot_field_position, &right_foot_field_position),
                    &expected_referee_position,
                );
                if distance_to_center < distance_to_center_threshhold {
                    Some((pose, distance_to_center))
                } else {
                    None
                }
            })
            .collect_vec();

        referee_pose_candidates
            .iter()
            .reduce(|a, b| if a.1 < b.1 { a } else { b })
            .unwrap()
            .0
            .clone()
    }

    pub fn interpret_pose(human_pose: &HumanPose) -> PoseType {
        let left_ear_keypoint = &human_pose.keypoints.left_ear;
        let right_ear_keypoint = &human_pose.keypoints.right_ear;
        let left_hand_keypoint = &human_pose.keypoints.left_hand;
        let right_hand_keypoint = &human_pose.keypoints.right_hand;
        if left_ear_keypoint.point.y > left_hand_keypoint.point.y
            || right_ear_keypoint.point.y > right_hand_keypoint.point.y
        {
            PoseType::OverheadArms
        } else {
            PoseType::default()
        }
    }
}
