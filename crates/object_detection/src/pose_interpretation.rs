use std::time::Duration;

use color_eyre::Result;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground, Pixel};
use framework::{AdditionalOutput, MainOutput};
use hardware::{NetworkInterface, PathsInterface};
use linear_algebra::{center, distance, Isometry2, Point2, Rotation2};
use projection::{camera_matrices::CameraMatrices, camera_matrix::CameraMatrix, Projection};
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
    rejected_human_poses: Input<Vec<HumanPose>, "rejected_human_poses">,
    accepted_human_poses: Input<Vec<HumanPose>, "accepted_human_poses">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "Control", "ground_to_field?">,
    expected_referee_position:
        Input<Option<Point2<Field>>, "Control", "expected_referee_position?">,
    fall_state: Input<FallState, "Control", "fall_state">,

    maximum_distance_to_referee_position:
        Parameter<f32, "pose_detection.maximum_distance_to_referee_position">,
    foot_z_offset: Parameter<f32, "pose_detection.foot_z_offset">,
    minimum_shoulder_angle: Parameter<f32, "pose_detection.minimum_shoulder_angle">,

    rejected_pose_kind_positions:
        AdditionalOutput<Vec<PoseKindPosition<Field>>, "rejected_pose_kind_positions">,
    accepted_pose_kind_positions:
        AdditionalOutput<Vec<PoseKindPosition<Field>>, "accepted_pose_kind_positions">,
    referee_pose_kind_position:
        AdditionalOutput<Option<PoseKindPosition<Field>>, "referee_pose_kind_position">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub referee_pose_kind: MainOutput<Option<PoseKind>>,
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
            context
                .rejected_pose_kind_positions
                .fill_if_subscribed(Vec::new);
            context
                .accepted_pose_kind_positions
                .fill_if_subscribed(Vec::new);
            context
                .referee_pose_kind_position
                .fill_if_subscribed(|| None);
            return Ok(MainOutputs {
                referee_pose_kind: None.into(),
            });
        };

        let referee_pose = get_position_filtered_pose(
            context.accepted_human_poses.clone(),
            context.camera_matrices.top.clone(),
            *context.maximum_distance_to_referee_position,
            ground_to_field.inverse() * expected_referee_position,
            *context.foot_z_offset,
        );

        let referee_pose_kind =
            referee_pose.map(|pose| interpret_pose(pose, *context.minimum_shoulder_angle));

        context.rejected_pose_kind_positions.fill_if_subscribed(|| {
            get_all_pose_kind_positions(
                context.rejected_human_poses,
                context.camera_matrices.top.clone(),
                context.ground_to_field,
                *context.foot_z_offset,
                *context.minimum_shoulder_angle,
            )
        });

        context.accepted_pose_kind_positions.fill_if_subscribed(|| {
            get_all_pose_kind_positions(
                context.accepted_human_poses,
                context.camera_matrices.top.clone(),
                context.ground_to_field,
                *context.foot_z_offset,
                *context.minimum_shoulder_angle,
            )
        });

        context.referee_pose_kind_position.fill_if_subscribed(|| {
            get_pose_kind_position(
                referee_pose,
                &context.camera_matrices.top,
                context.ground_to_field,
                *context.foot_z_offset,
                *context.minimum_shoulder_angle,
            )
        });

        Ok(MainOutputs {
            referee_pose_kind: referee_pose_kind.into(),
        })
    }
}

fn get_position_filtered_pose(
    filtered_poses: Vec<HumanPose>,
    camera_matrix_top: CameraMatrix,
    maximum_distance_to_referee_position: f32,
    expected_referee_position: Point2<Ground>,
    foot_z_offset: f32,
) -> Option<HumanPose> {
    let located_pose_candidate: RefereePoseCandidate = get_closest_referee_pose(
        filtered_poses,
        camera_matrix_top,
        expected_referee_position,
        foot_z_offset,
    )?;

    if located_pose_candidate.distance_to_referee_position < maximum_distance_to_referee_position {
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

fn interpret_pose(human_pose: HumanPose, minimum_shoulder_angle: f32) -> PoseKind {
    if is_above_head_arms_pose(human_pose.keypoints, minimum_shoulder_angle) {
        PoseKind::AboveHeadArms
    } else {
        PoseKind::UndefinedPose
    }
}

fn is_above_head_arms_pose(keypoints: Keypoints, minimum_shoulder_angle: f32) -> bool {
    are_hands_above_shoulder(&keypoints)
        && is_right_shoulder_angled_up(
            &keypoints,
            keypoints.right_shoulder.point,
            keypoints.right_elbow.point,
            minimum_shoulder_angle,
        )
        && is_left_shoulder_angled_up(
            &keypoints,
            keypoints.left_shoulder.point,
            keypoints.left_elbow.point,
            minimum_shoulder_angle,
        )
}

fn are_hands_above_shoulder(keypoints: &Keypoints) -> bool {
    keypoints.left_shoulder.point.y() > keypoints.left_hand.point.y()
        && keypoints.right_shoulder.point.y() > keypoints.right_hand.point.y()
}

fn is_right_shoulder_angled_up(
    keypoints: &Keypoints,
    shoulder_point: Point2<Pixel>,
    elbow_point: Point2<Pixel>,
    minimum_shoulder_angle: f32,
) -> bool {
    let left_to_right_shoulder =
        keypoints.right_shoulder.point.coords() - keypoints.left_shoulder.point.coords();

    Rotation2::rotation_between(left_to_right_shoulder, elbow_point - shoulder_point).angle()
        > minimum_shoulder_angle
}

fn is_left_shoulder_angled_up(
    keypoints: &Keypoints,
    shoulder_point: Point2<Pixel>,
    elbow_point: Point2<Pixel>,
    minimum_shoulder_angle: f32,
) -> bool {
    let right_to_left_shoulder =
        keypoints.left_shoulder.point.coords() - keypoints.right_shoulder.point.coords();

    Rotation2::rotation_between(elbow_point - shoulder_point, right_to_left_shoulder).angle()
        > minimum_shoulder_angle
}

fn get_all_pose_kind_positions(
    poses: &[HumanPose],
    camera_matrix_top: CameraMatrix,
    ground_to_field: Option<&Isometry2<Ground, Field>>,
    foot_z_offset: f32,
    minimum_shoulder_angle: f32,
) -> Vec<PoseKindPosition<Field>> {
    poses
        .iter()
        .filter_map(|pose: &HumanPose| {
            get_pose_kind_position(
                Some(*pose),
                &camera_matrix_top,
                ground_to_field,
                foot_z_offset,
                minimum_shoulder_angle,
            )
        })
        .collect()
}

fn get_pose_kind_position(
    pose: Option<HumanPose>,
    camera_matrix_top: &CameraMatrix,
    ground_to_field: Option<&Isometry2<Ground, Field>>,
    foot_z_offset: f32,
    minimum_shoulder_angle: f32,
) -> Option<PoseKindPosition<Field>> {
    let left_foot_ground_position = camera_matrix_top
        .pixel_to_ground_with_z(pose?.keypoints.left_foot.point, foot_z_offset)
        .ok()?;
    let right_foot_ground_position = camera_matrix_top
        .pixel_to_ground_with_z(pose?.keypoints.right_foot.point, foot_z_offset)
        .ok()?;
    let interpreted_pose_kind = interpret_pose(pose?, minimum_shoulder_angle);
    Some(PoseKindPosition {
        pose_kind: interpreted_pose_kind,
        position: center(
            ground_to_field? * left_foot_ground_position,
            ground_to_field? * right_foot_ground_position,
        ),
    })
}
