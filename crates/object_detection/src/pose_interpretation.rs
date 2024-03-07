use std::f32::consts::PI;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use hardware::TimeInterface;
use nalgebra::center;
use nalgebra::distance;
use nalgebra::point;
use nalgebra::vector;
use nalgebra::Isometry2;
use nalgebra::Point2;
use ordered_float::NotNan;
use projection::Projection;
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, Penalty, PlayerNumber, Team};
use types::camera_matrix::CameraMatrices;
use types::camera_matrix::CameraMatrix;
use types::line::Line;
use types::pose_detection::Keypoints;
use types::{
    field_dimensions::FieldDimensions, initial_pose::InitialPose, players::Players,
    pose_detection::HumanPose, pose_types::PoseType, ycbcr422_image::YCbCr422Image,
};

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
    keypoint_confidence_threshold:
        Parameter<f32, "detection.$cycler_instance.keypoint_confidence_threshold">,
    initial_poses: Parameter<Players<InitialPose>, "localization.initial_poses">,
    distance_to_referee_position_threshhold:
        Parameter<f32, "detection.$cycler_instance.distance_to_referee_position_threshhold">,
    expected_referee_position:
        Parameter<Point2<f32>, "detection.$cycler_instance.expected_referee_position">,
    foot_z_offset: Parameter<f32, "detection.$cycler_instance.foot_z_offset">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_referee_pose_type: MainOutput<PoseType>,
    pub detected_pose_types: MainOutput<Vec<(PoseType, Point2<f32>)>>,
}

impl PoseInterpretation {
    pub fn new(_context: CreationContext<impl PathsInterface>) -> Result<Self> {
        Ok(PoseInterpretation {
            interpreted_pose_type: PoseType::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        let interpreted_pose_types: Vec<(PoseType, Point2<f32>)> = Self::get_all_pose_types(
            context.human_poses.clone(),
            context.camera_matrices.top.clone(),
            context.robot_to_field,
            *context.foot_z_offset,
            *context.keypoint_confidence_threshold,
        );
        let half_field_size = vector!(
            context.field_dimensions.width / 2.0,
            context.field_dimensions.length / 2.0
        );

        let expected_referee_position = *context
            .expected_referee_position
            .coords
            .component_mul(&half_field_size);

        let referee_pose = Self::get_referee_pose_type(
            context.human_poses.clone(),
            context.camera_matrices.top.clone(),
            context.robot_to_field,
            *context.distance_to_referee_position_threshhold,
            point!(expected_referee_position.x, expected_referee_position.y),
            *context.foot_z_offset,
        );
        let pose_type = Self::interpret_pose(referee_pose, *context.keypoint_confidence_threshold);

        Ok(MainOutputs {
            detected_referee_pose_type: pose_type.into(),
            detected_pose_types: interpreted_pose_types.into(),
        })
    }

    pub fn get_all_pose_types(
        poses: Vec<HumanPose>,
        camera_matrix_top: CameraMatrix,
        robot_to_field: Option<&Isometry2<f32>>,
        foot_z_offset: f32,
        keypoint_confidence_threshold: f32,
    ) -> Vec<(PoseType, Point2<f32>)> {
        let pose_type_tuple = poses
            .iter()
            .filter_map(|pose| {
                if let Some(robot_to_field) = robot_to_field {
                    let left_foot_ground_position = camera_matrix_top
                        .pixel_to_ground_with_z(pose.keypoints.left_foot.point, foot_z_offset)
                        .ok();
                    let right_foot_ground_position = camera_matrix_top
                        .pixel_to_ground_with_z(pose.keypoints.right_foot.point, foot_z_offset)
                        .ok();
                    if let Some((left_foot_ground_position, right_foot_ground_position)) =
                        left_foot_ground_position.zip(right_foot_ground_position)
                    {
                        let interpreted_pose =
                            Self::interpret_pose(Some(pose.clone()), keypoint_confidence_threshold);
                        Some((
                            interpreted_pose,
                            center(
                                &(robot_to_field * &left_foot_ground_position),
                                &(robot_to_field * &right_foot_ground_position),
                            ),
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        pose_type_tuple
    }

    pub fn get_referee_pose_type(
        poses: Vec<HumanPose>,
        camera_matrix_top: CameraMatrix,
        robot_to_field: Option<&Isometry2<f32>>,
        distance_to_referee_position_threshhold: f32,
        expected_referee_position: Point2<f32>,
        foot_z_offset: f32,
    ) -> Option<HumanPose> {
        let pose_type_tuple = poses
            // Get all poses that are near the referee position within a threshhold
            .iter()
            .filter_map(|pose| {
                if let Some(robot_to_field) = robot_to_field {
                    let left_foot_ground_position = camera_matrix_top
                        .pixel_to_ground_with_z(pose.keypoints.left_foot.point, foot_z_offset)
                        .ok();
                    let right_foot_ground_position = camera_matrix_top
                        .pixel_to_ground_with_z(pose.keypoints.right_foot.point, foot_z_offset)
                        .ok();
                    if let Some((left_foot_ground_position, right_foot_ground_position)) =
                        left_foot_ground_position.zip(right_foot_ground_position)
                    {
                        let distance_to_referee_position = distance(
                            &center(
                                &(robot_to_field * &left_foot_ground_position),
                                &(robot_to_field * &right_foot_ground_position),
                            ),
                            &expected_referee_position,
                        );
                        Some((pose, distance_to_referee_position))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .min_by_key(|(_, distance)| NotNan::new(*distance).unwrap());

        match pose_type_tuple {
            Some((pose, distance_to_referee_position))
                if distance_to_referee_position < distance_to_referee_position_threshhold =>
            {
                Some(pose.clone())
            }
            _ => None,
        }
    }

    pub fn interpret_pose(
        human_pose: Option<HumanPose>,
        keypoint_confidence_threshold: f32,
    ) -> PoseType {
        match human_pose {
            Some(pose)
                if Self::check_overarms(pose.keypoints.clone(), keypoint_confidence_threshold) =>
            {
                PoseType::OverheadArms
            }
            _ => PoseType::default(),
        }
    }

    pub fn check_overarms(keypoints: Keypoints, keypoint_confidence_threshold: f32) -> bool {
        let are_hands_visible = keypoints.left_hand.confidence > keypoint_confidence_threshold
            && keypoints.right_hand.confidence > keypoint_confidence_threshold;
        let are_hands_over_shoulder = keypoints.left_shoulder.point.y > keypoints.left_hand.point.y
            || keypoints.right_shoulder.point.y > keypoints.right_hand.point.y;

        let shoulder_line = Line(
            keypoints.right_shoulder.point,
            keypoints.left_shoulder.point,
        );
        let left_shoulder_elbow_line =
            Line(keypoints.left_shoulder.point, keypoints.left_elbow.point);
        let right_shoulder_elbow_line =
            Line(keypoints.right_shoulder.point, keypoints.right_elbow.point);
        let is_shoulder_angled_up = shoulder_line.angle(left_shoulder_elbow_line) > PI
            && shoulder_line.angle(right_shoulder_elbow_line) < PI;

        if are_hands_visible {
            are_hands_over_shoulder
        } else {
            is_shoulder_angled_up
        }
    }
}
