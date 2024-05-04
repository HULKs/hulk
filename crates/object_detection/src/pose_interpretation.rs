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
    pose_detection::{HumanPose, Keypoint, Keypoints, RefereePoseCandidate},
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
    overall_keypoint_confidence_threshold:
        Parameter<f32, "object_detection.$cycler_instance.overall_keypoint_confidence_threshold">,
    visual_referee_keypoint_confidence_threshold: Parameter<
        f32,
        "object_detection.$cycler_instance.visual_referee_keypoint_confidence_threshold",
    >,
    distance_to_referee_position_threshold:
        Parameter<f32, "object_detection.$cycler_instance.distance_to_referee_position_threshold">,
    foot_z_offset: Parameter<f32, "object_detection.$cycler_instance.foot_z_offset">,
    shoulder_angle_threshold:
        Parameter<f32, "object_detection.$cycler_instance.shoulder_angle_threshold">,

    raw_pose_kinds: AdditionalOutput<Vec<PoseKindPosition<Field>>, "raw_pose_kinds">,
    filtered_pose_kinds: AdditionalOutput<Vec<PoseKindPosition<Field>>, "filtered_pose_kinds">,
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
            context.raw_pose_kinds.fill_if_subscribed(Vec::new);
            context.filtered_pose_kinds.fill_if_subscribed(Vec::new);
            return Ok(MainOutputs {
                detected_referee_pose_kind: None.into(),
            });
        };

        let referee_pose = get_referee_pose(
            context.human_poses.clone(),
            *context.overall_keypoint_confidence_threshold,
            *context.visual_referee_keypoint_confidence_threshold,
            context.camera_matrices.top.clone(),
            *context.distance_to_referee_position_threshold,
            ground_to_field.inverse() * expected_referee_position,
            *context.foot_z_offset,
        );

        let pose_kind = interpret_pose(referee_pose, *context.shoulder_angle_threshold);

        context.raw_pose_kinds.fill_if_subscribed(|| {
            get_all_pose_kinds(
                context.human_poses,
                0.0,
                0.0,
                context.camera_matrices.top.clone(),
                context.ground_to_field,
                *context.foot_z_offset,
                *context.shoulder_angle_threshold,
            )
        });

        context.filtered_pose_kinds.fill_if_subscribed(|| {
            get_all_pose_kinds(
                context.human_poses,
                *context.overall_keypoint_confidence_threshold,
                *context.visual_referee_keypoint_confidence_threshold,
                context.camera_matrices.top.clone(),
                context.ground_to_field,
                *context.foot_z_offset,
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
    overall_keypoint_confidence_threshold: f32,
    visual_referee_keypoint_confidence_threshold: f32,
    camera_matrix_top: CameraMatrix,
    distance_to_referee_position_threshold: f32,
    expected_referee_position: Point2<Ground>,
    foot_z_offset: f32,
) -> Option<HumanPose> {
    let overall_pose_candidates =
        filter_poses_by_overall_confidence(poses, overall_keypoint_confidence_threshold);

    let visual_referee_pose_candidates = filter_poses_by_visual_referee_confidence(
        overall_pose_candidates,
        visual_referee_keypoint_confidence_threshold,
    );

    let located_pose_candidate: RefereePoseCandidate = get_closest_referee_pose(
        visual_referee_pose_candidates,
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

fn filter_poses_by_overall_confidence(
    poses: Vec<HumanPose>,
    overall_keypoint_confidence_threshold: f32,
) -> Vec<HumanPose> {
    poses
        .iter()
        .filter(|pose| {
            pose.keypoints
                .iter()
                .all(|keypoint| keypoint.confidence > overall_keypoint_confidence_threshold)
        })
        .copied()
        .collect()
}

fn filter_poses_by_visual_referee_confidence(
    pose_candidates: Vec<HumanPose>,
    visual_referee_keypoint_confidence_threshold: f32,
) -> Vec<HumanPose> {
    pose_candidates
        .iter()
        .filter(|pose| {
            let visual_referee_keypoint_indices = [0, 1, 5, 6, 7, 8, 9, 10, 15, 16];
            let visual_referee_keypoints: Vec<Keypoint> = visual_referee_keypoint_indices
                .iter()
                .map(|&i| pose.keypoints[i])
                .collect();
            visual_referee_keypoints
                .iter()
                .all(|keypoint| keypoint.confidence > visual_referee_keypoint_confidence_threshold)
        })
        .copied()
        .collect()
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

fn interpret_pose(
    human_pose: Option<HumanPose>,
    shoulder_angle_threshold: f32,
) -> Option<PoseKind> {
    if is_above_head_arms_pose(human_pose?.keypoints, shoulder_angle_threshold) {
        Some(PoseKind::AboveHeadArms)
    } else {
        None
    }
}

fn is_above_head_arms_pose(keypoints: Keypoints, shoulder_angle_threshold: f32) -> bool {
    are_hands_above_shoulder(&keypoints)
        && is_shoulder_angled_up(
            &keypoints,
            keypoints.right_shoulder.point,
            keypoints.right_elbow.point,
            shoulder_angle_threshold,
        )
        && is_shoulder_angled_up(
            &keypoints,
            keypoints.left_shoulder.point,
            keypoints.left_elbow.point,
            shoulder_angle_threshold,
        )
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
    overall_keypoint_confidence_threshold: f32,
    visual_referee_keypoint_confidence_threshold: f32,
    camera_matrix_top: CameraMatrix,
    ground_to_field: Option<&Isometry2<Ground, Field>>,
    foot_z_offset: f32,
    shoulder_angle_threshold: f32,
) -> Vec<PoseKindPosition<Field>> {
    let Some(ground_to_field) = ground_to_field else {
        return Vec::new();
    };
    let overall_filtered_poses =
        filter_poses_by_overall_confidence(poses.to_vec(), overall_keypoint_confidence_threshold);
    let visual_referee_filtered_poses = filter_poses_by_overall_confidence(
        overall_filtered_poses,
        visual_referee_keypoint_confidence_threshold,
    );

    visual_referee_filtered_poses
        .iter()
        .filter_map(|pose| {
            let left_foot_ground_position = camera_matrix_top
                .pixel_to_ground_with_z(pose.keypoints.left_foot.point, foot_z_offset)
                .ok()?;
            let right_foot_ground_position = camera_matrix_top
                .pixel_to_ground_with_z(pose.keypoints.right_foot.point, foot_z_offset)
                .ok()?;
            let interpreted_pose_kind = interpret_pose(Some(*pose), shoulder_angle_threshold)?;
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
