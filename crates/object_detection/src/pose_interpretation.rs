use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground, Pixel};
use framework::{AdditionalOutput, MainOutput};
use hardware::{NetworkInterface, PathsInterface};
use linear_algebra::{center, distance, Isometry2, Point2, Transform};
use ordered_float::NotNan;
use projection::{camera_matrices::CameraMatrices, camera_matrix::CameraMatrix, Projection};
use serde::{Deserialize, Serialize};
use spl_network_messages::PlayerNumber;
use types::{
    fall_state::FallState,
    pose_detection::{HumanPose, Keypoints, RefereePoseCandidate},
    pose_kinds::{PoseKind, PoseKindPosition},
};

#[derive(Deserialize, Serialize)]
pub struct PoseInterpretation {}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
    time_to_reach_kick_position: CyclerState<Duration, "time_to_reach_kick_position">,

    camera_matrices: RequiredInput<Option<CameraMatrices>, "Control", "camera_matrices?">,
    human_poses: Input<Vec<HumanPose>, "human_poses">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "Control", "ground_to_field?">,
    expected_referee_position:
        Input<Option<Point2<Field>>, "Control", "expected_referee_position?">,
    fall_state: Input<FallState, "Control", "fall_state">,

    player_number: Parameter<PlayerNumber, "player_number">,
    keypoint_confidence_threshold:
        Parameter<f32, "object_detection.$cycler_instance.keypoint_confidence_threshold">,
    distance_to_referee_position_threshold:
        Parameter<f32, "object_detection.$cycler_instance.distance_to_referee_position_threshold">,
    foot_z_offset: Parameter<f32, "object_detection.$cycler_instance.foot_z_offset">,
    shoulder_angle_threshold:
        Parameter<f32, "object_detection.$cycler_instance.shoulder_angle_threshold">,

    detected_pose_kinds: AdditionalOutput<Vec<PoseKindPosition<Field>>, "detected_pose_kinds">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_referee_pose_kind: MainOutput<Option<PoseKind>>,
}

impl PoseInterpretation {
    pub fn new(_creation_context: CreationContext<impl PathsInterface>) -> Result<Self> {
        Ok(PoseInterpretation {})
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        let (Some(ground_to_field), Some(expected_referee_position)) =
            (context.ground_to_field, context.expected_referee_position)
        else {
            context.detected_pose_kinds.fill_if_subscribed(Vec::new);
            return Ok(MainOutputs {
                detected_referee_pose_kind: None.into(),
            });
        };

        let referee_pose = get_referee_pose(
            context.human_poses.clone(),
            *context.keypoint_confidence_threshold,
            context.camera_matrices.top.clone(),
            *context.distance_to_referee_position_threshold,
            ground_to_field.inverse() * expected_referee_position,
            *context.foot_z_offset,
        );

        let pose_kind = interpret_pose(
            referee_pose,
            *context.keypoint_confidence_threshold,
            *context.shoulder_angle_threshold,
        );

        context.detected_pose_kinds.fill_if_subscribed(|| {
            get_all_pose_kinds(
                context.human_poses,
                context.camera_matrices.top.clone(),
                context.ground_to_field,
                *context.foot_z_offset,
                *context.keypoint_confidence_threshold,
                *context.shoulder_angle_threshold,
            )
        });

        Ok(MainOutputs {
            detected_referee_pose_kind: pose_kind.into(),
        })
    }
}

fn get_referee_pose(
    poses: Vec<HumanPose>,
    keypoint_confidence_threshold: f32,
    camera_matrix_top: CameraMatrix,
    distance_to_referee_position_threshold: f32,
    expected_referee_position: Point2<Ground>,
    foot_z_offset: f32,
) -> Option<HumanPose> {
    let confident_pose_candidates =
        filter_poses_by_confidence(poses, keypoint_confidence_threshold);

    let located_pose_candidate: RefereePoseCandidate = get_closest_referee_pose(
        confident_pose_candidates,
        camera_matrix_top,
        expected_referee_position,
        foot_z_offset,
    )?;

    if located_pose_candidate.distance_to_referee_position < distance_to_referee_position_threshold
    {
        Some(located_pose_candidate.pose)
    } else {
        None
    }
}

fn get_closest_referee_pose(
    poses: Vec<HumanPose>,
    camera_matrix_top: CameraMatrix,
    expected_referee_position: Point2<Ground>,
    foot_z_offset: f32,
) -> Option<RefereePoseCandidate> {
    poses
        .iter()
        .filter_map(|pose| {
            let left_foot_ground_position = camera_matrix_top
                .pixel_to_ground_with_z(pose.keypoints.left_foot.point, foot_z_offset)
                .ok()?;
            let right_foot_ground_position = camera_matrix_top
                .pixel_to_ground_with_z(pose.keypoints.right_foot.point, foot_z_offset)
                .ok()?;
            let distance_to_referee_position = distance(
                center(left_foot_ground_position, right_foot_ground_position),
                expected_referee_position,
            );
            Some(RefereePoseCandidate {
                pose: *pose,
                distance_to_referee_position,
            })
        })
        .min_by_key(|pose_candidate| {
            NotNan::new(pose_candidate.distance_to_referee_position).unwrap()
        })
}

fn filter_poses_by_confidence(
    pose_candidates: Vec<HumanPose>,
    keypoint_confidence_threshold: f32,
) -> Vec<HumanPose> {
    pose_candidates
        .iter()
        .filter(|pose| {
            pose.keypoints
                .iter()
                .all(|keypoint| keypoint.confidence > keypoint_confidence_threshold)
        })
        .copied()
        .collect()
}

fn interpret_pose(
    human_pose: Option<HumanPose>,
    keypoint_confidence_threshold: f32,
    shoulder_angle_threshold: f32,
) -> Option<PoseKind> {
    if is_above_head_arms_pose(
        human_pose?.keypoints,
        keypoint_confidence_threshold,
        shoulder_angle_threshold,
    ) {
        Some(PoseKind::AboveHeadArms)
    } else {
        None
    }
}

fn is_above_head_arms_pose(
    keypoints: Keypoints,
    keypoint_confidence_threshold: f32,
    shoulder_angle_threshold: f32,
) -> bool {
    if are_hands_visible(&keypoints, keypoint_confidence_threshold) {
        are_hands_above_shoulder(&keypoints)
    } else {
        is_shoulder_angled_up(
            &keypoints,
            keypoints.right_shoulder.point,
            keypoints.right_elbow.point,
            shoulder_angle_threshold,
        ) && is_shoulder_angled_up(
            &keypoints,
            keypoints.left_shoulder.point,
            keypoints.left_elbow.point,
            shoulder_angle_threshold,
        )
    }
}

fn are_hands_visible(keypoints: &Keypoints, keypoint_confidence_threshold: f32) -> bool {
    keypoints.left_hand.confidence > keypoint_confidence_threshold
        && keypoints.right_hand.confidence > keypoint_confidence_threshold
}

fn are_hands_above_shoulder(keypoints: &Keypoints) -> bool {
    keypoints.left_shoulder.point.y() > keypoints.left_hand.point.y()
        && keypoints.right_shoulder.point.y() > keypoints.right_hand.point.y()
}

fn is_shoulder_angled_up(
    keypoints: &Keypoints,
    shoulder_point: Point2<Pixel>,
    elbow_point: Point2<Pixel>,
    shoulder_angle_threshold: f32,
) -> bool {
    struct RotatedPixel;

    let left_to_right_shoulder =
        keypoints.right_shoulder.point.coords() - keypoints.left_shoulder.point.coords();
    let shoulder_line_angle = f32::atan2(left_to_right_shoulder.y(), left_to_right_shoulder.x());
    let shoulder_rotation =
        Transform::<Pixel, RotatedPixel, nalgebra::Isometry2<_>>::rotation(shoulder_line_angle);

    let shoulder = shoulder_rotation * shoulder_point;
    let elbow = shoulder_rotation * elbow_point;

    let shoulder_to_elbow = elbow.coords() - shoulder.coords();

    f32::atan2(shoulder_to_elbow.y(), shoulder_to_elbow.x()) > shoulder_angle_threshold
}

fn get_all_pose_kinds(
    poses: &[HumanPose],
    camera_matrix_top: CameraMatrix,
    ground_to_field: Option<&Isometry2<Ground, Field>>,
    foot_z_offset: f32,
    keypoint_confidence_threshold: f32,
    shoulder_angle_threshold: f32,
) -> Vec<PoseKindPosition<Field>> {
    let Some(ground_to_field) = ground_to_field else {
        return Vec::new();
    };
    poses
        .iter()
        .filter_map(|pose| {
            let left_foot_ground_position = camera_matrix_top
                .pixel_to_ground_with_z(pose.keypoints.left_foot.point, foot_z_offset)
                .ok()?;
            let right_foot_ground_position = camera_matrix_top
                .pixel_to_ground_with_z(pose.keypoints.right_foot.point, foot_z_offset)
                .ok()?;
            let interpreted_pose_kind = interpret_pose(
                Some(*pose),
                keypoint_confidence_threshold,
                shoulder_angle_threshold,
            )?;
            Some(PoseKindPosition {
                pose_kind: interpreted_pose_kind,
                position: center(
                    ground_to_field * left_foot_ground_position,
                    ground_to_field * right_foot_ground_position,
                ),
            })
        })
        .collect()
}
