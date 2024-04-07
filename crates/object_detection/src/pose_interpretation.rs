use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Field;
use coordinate_systems::Ground;
use coordinate_systems::Pixel;
use framework::AdditionalOutput;
use framework::MainOutput;
use hardware::PathsInterface;
use hardware::TimeInterface;
use linear_algebra::center;
use linear_algebra::distance;
use linear_algebra::Isometry2;
use linear_algebra::Point2;
use linear_algebra::Transform;
use ordered_float::NotNan;
use projection::Projection;
use serde::{Deserialize, Serialize};
use types::camera_matrix::CameraMatrices;
use types::camera_matrix::CameraMatrix;
use types::cycle_time::CycleTime;
use types::field_dimensions::FieldDimensions;
use types::initial_pose::InitialPose;
use types::players::Players;
use types::pose_detection::Keypoints;
use types::ycbcr422_image::YCbCr422Image;
use types::{pose_detection::HumanPose, pose_types::PoseType};

#[derive(Deserialize, Serialize)]
pub struct PoseInterpretation {}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,

    camera_matrices: RequiredInput<Option<CameraMatrices>, "Control", "camera_matrices?">,
    cycle_time: Input<CycleTime, "Control", "cycle_time">,
    human_poses: Input<Vec<HumanPose>, "human_poses">,
    image: Input<YCbCr422Image, "image">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "Control", "ground_to_field?">,
    ground_to_field_of_home_after_coin_toss_before_second_half: Input<
        Option<Isometry2<Ground, Field>>,
        "Control",
        "ground_to_field_of_home_after_coin_toss_before_second_half?",
    >,

    expected_referee_position: Input<Point2<Ground>, "Control", "expected_referee_position">,
    shoulder_angle_left: AdditionalOutput<f32, "shoulder_angle_left">,
    shoulder_angle_right: AdditionalOutput<f32, "shoulder_angle_right">,
    distance_to_referee: AdditionalOutput<f32, "distance_to_referee">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    keypoint_confidence_threshold:
        Parameter<f32, "detection.$cycler_instance.keypoint_confidence_threshold">,
    initial_poses: Parameter<Players<InitialPose>, "localization.initial_poses">,
    distance_to_referee_position_threshhold:
        Parameter<f32, "detection.$cycler_instance.distance_to_referee_position_threshhold">,
    foot_z_offset: Parameter<f32, "detection.$cycler_instance.foot_z_offset">,
    shoulder_angle_threshhold:
        Parameter<f32, "detection.$cycler_instance.shoulder_angle_threshhold">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_referee_over_arms_pose_time: MainOutput<Option<SystemTime>>,
    pub detected_pose_types: MainOutput<Vec<(PoseType, Point2<Field>)>>,
}

impl PoseInterpretation {
    pub fn new(_context: CreationContext<impl PathsInterface>) -> Result<Self> {
        Ok(PoseInterpretation {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        let interpreted_pose_types: Vec<(PoseType, Point2<Field>)> = Self::get_all_pose_types(
            context.human_poses.clone(),
            context.camera_matrices.top.clone(),
            context.ground_to_field,
            *context.foot_z_offset,
            *context.keypoint_confidence_threshold,
            *context.shoulder_angle_threshhold,
        );

        let referee_pose = Self::get_referee_pose_type(
            context.human_poses.clone(),
            context.camera_matrices.top.clone(),
            *context.distance_to_referee_position_threshhold,
            *context.expected_referee_position,
            *context.foot_z_offset,
        );

        let pose_type = Self::interpret_pose(
            referee_pose,
            *context.keypoint_confidence_threshold,
            *context.shoulder_angle_threshhold,
        );

        if let PoseType::OverheadArms = pose_type {
            let detected_referee_over_arms_pose_time = Some(context.cycle_time.start_time);
        } else {
            let detected_referee_over_arms_pose_time = None;
        }

        Ok(MainOutputs {
            detected_referee_over_arms_pose_time: pose_type.into(),
            detected_pose_types: interpreted_pose_types.into(),
        })
    }

    pub fn get_all_pose_types(
        poses: Vec<HumanPose>,
        camera_matrix_top: CameraMatrix,
        ground_to_field: Option<&Isometry2<Ground, Field>>,
        foot_z_offset: f32,
        keypoint_confidence_threshold: f32,
        shoulder_angle_threshhold: f32,
    ) -> Vec<(PoseType, Point2<Field>)> {
        let pose_type_tuple = poses
            .iter()
            .filter_map(|pose| {
                if let Some(ground_to_field) = ground_to_field {
                    let left_foot_ground_position = camera_matrix_top
                        .pixel_to_ground_with_z(pose.keypoints.left_foot.point, foot_z_offset)
                        .ok();
                    let right_foot_ground_position = camera_matrix_top
                        .pixel_to_ground_with_z(pose.keypoints.right_foot.point, foot_z_offset)
                        .ok();
                    if let Some((left_foot_ground_position, right_foot_ground_position)) =
                        left_foot_ground_position.zip(right_foot_ground_position)
                    {
                        let interpreted_pose = Self::interpret_pose(
                            Some(pose.clone()),
                            keypoint_confidence_threshold,
                            shoulder_angle_threshhold,
                        );
                        Some((
                            interpreted_pose,
                            center(
                                ground_to_field * left_foot_ground_position,
                                ground_to_field * right_foot_ground_position,
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
        distance_to_referee_position_threshhold: f32,
        expected_referee_position: Point2<Ground>,
        foot_z_offset: f32,
    ) -> Option<HumanPose> {
        let pose_type_tuple = poses
            // Get all poses that are near the referee position within a threshhold
            .iter()
            .filter_map(|pose| {
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
                        center(left_foot_ground_position, right_foot_ground_position),
                        expected_referee_position,
                    );
                    Some((pose, distance_to_referee_position))
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
        shoulder_angle_threshhold: f32,
    ) -> PoseType {
        match human_pose {
            Some(pose)
                if Self::is_overarms(
                    pose.keypoints.clone(),
                    keypoint_confidence_threshold,
                    shoulder_angle_threshhold,
                ) =>
            {
                PoseType::OverheadArms
            }
            _ => PoseType::default(),
        }
    }

    pub fn is_overarms(
        keypoints: Keypoints,
        keypoint_confidence_threshold: f32,
        shoulder_angle_threshhold: f32,
    ) -> bool {
        struct RotatedPixel;

        let are_hands_visible = keypoints.left_hand.confidence > keypoint_confidence_threshold
            && keypoints.right_hand.confidence > keypoint_confidence_threshold;
        let are_hands_over_shoulder = keypoints.left_shoulder.point.y()
            > keypoints.left_hand.point.y()
            && keypoints.right_shoulder.point.y() > keypoints.right_hand.point.y();

        let left_to_right_shoulder =
            keypoints.right_shoulder.point.coords() - keypoints.left_shoulder.point.coords();
        let shoulder_line_angle =
            f32::atan2(left_to_right_shoulder.y(), left_to_right_shoulder.x());
        let shoulder_rotation =
            Transform::<Pixel, RotatedPixel, nalgebra::Isometry2<_>>::rotation(shoulder_line_angle);
        let left_shoulder = shoulder_rotation * keypoints.left_shoulder.point;
        let right_shoulder = shoulder_rotation * keypoints.right_shoulder.point;
        let left_elbow = shoulder_rotation * keypoints.left_elbow.point;
        let right_elbow = shoulder_rotation * keypoints.right_elbow.point;
        let left_shoulder_to_elbow = left_elbow.coords() - left_shoulder.coords();
        let right_shoulder_to_elbow = right_elbow.coords() - right_shoulder.coords();
        let is_left_shoulder_angled_up =
            f32::atan2(right_shoulder_to_elbow.y(), right_shoulder_to_elbow.x())
                > shoulder_angle_threshhold;
        let is_right_shoulder_angled_up =
            f32::atan2(left_shoulder_to_elbow.y(), -left_shoulder_to_elbow.x())
                > shoulder_angle_threshhold;

        if are_hands_visible {
            are_hands_over_shoulder
        } else {
            is_right_shoulder_angled_up && is_left_shoulder_angled_up
        }
    }
}
