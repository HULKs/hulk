use std::{future::ready, time::Duration};

use color_eyre::Result;
use coordinate_systems::{Camera, Field, Robot};
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::{cache::Cache, parameter::NodeParameters, pubsub::Publisher, time::Time};
use ros_z_streams::FutureItem;
use types::{
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
    visual_localization::{AssociationPoseHint, GlobalLocalizationDebug, VisualLocalizationFrame},
};

use crate::{
    FieldMarkAssociationState, GlobalVisualLocalization,
    parameters::FieldMarkAssociationParameters, robot_to_camera,
};

const MAX_CAMERA_MATRIX_TIME_DISTANCE: Duration = Duration::from_millis(100);

type DetectedObjects = TimeWrapper<Vec<Object<RobocupObjectLabel>>>;
type DetectedObjectsItem<'a> = FutureItem<'a, (Option<DetectedObjects>,)>;

pub(crate) struct DetectionProcessingContext<'a> {
    pub(crate) parameters: &'a NodeParameters<FieldMarkAssociationParameters>,
    pub(crate) camera_matrix_cache: &'a Cache<TimeWrapper<CameraMatrix>>,
    pub(crate) field_dimensions_cache: &'a Cache<FieldDimensions>,
    pub(crate) localization_cache: &'a Cache<TimeWrapper<Option<AssociationPoseHint>>>,
    pub(crate) primary_state_cache: &'a Cache<PrimaryState>,
    pub(crate) association_state: &'a mut FieldMarkAssociationState,
    pub(crate) associations_publisher: &'a Publisher<TimeWrapper<VisualLocalizationFrame>>,
    pub(crate) global_localization_publisher: &'a Publisher<Option<GlobalLocalizationDebug>>,
}

struct PreparedDetectionFrame {
    image_time: Time,
    objects: Vec<Object<RobocupObjectLabel>>,
    camera_matrix: CameraMatrix,
    robot_to_camera: Isometry3<Robot, Camera>,
    field_dimensions: FieldDimensions,
    pose_hint: Option<Isometry3<Robot, Field>>,
    parameters: FieldMarkAssociationParameters,
    include_debug: bool,
}

struct ProcessedDetectionFrame {
    image_time: Time,
    robot_to_camera: Isometry3<Robot, Camera>,
    localization: GlobalVisualLocalization,
}

pub(crate) async fn process_detected_objects(
    item: DetectedObjectsItem<'_>,
    ctx: DetectionProcessingContext<'_>,
) -> Result<()> {
    for (image_time, (objects,)) in item.persistent {
        let Some(processed_frame) =
            tokio::task::block_in_place(|| -> Result<Option<ProcessedDetectionFrame>> {
                if association_is_damping(ctx.primary_state_cache) {
                    ctx.association_state.reset_for_damping();
                    return Ok(None);
                }

                let Some(frame) = prepare_detection_frame(image_time, objects, &ctx) else {
                    return Ok(None);
                };
                let image_time = frame.image_time;
                let robot_to_camera = frame.robot_to_camera;

                let state = std::mem::take(ctx.association_state);
                let (next_state, localization) = associate_detection_frame(state, frame)?;
                *ctx.association_state = next_state;

                if association_is_damping(ctx.primary_state_cache) {
                    ctx.association_state.reset_for_damping();
                    return Ok(None);
                }

                Ok(Some(ProcessedDetectionFrame {
                    image_time,
                    robot_to_camera,
                    localization,
                }))
            })?
        else {
            continue;
        };

        publish_localization_frame(
            ctx.associations_publisher,
            ctx.global_localization_publisher,
            processed_frame,
        )
        .await?;
    }
    Ok(())
}

fn prepare_detection_frame(
    image_time: Time,
    objects: Option<DetectedObjects>,
    ctx: &DetectionProcessingContext<'_>,
) -> Option<PreparedDetectionFrame> {
    let camera_matrix = ctx.camera_matrix_cache.get_nearest(image_time)?;
    if !camera_matrix_is_fresh(&camera_matrix, image_time) {
        return None;
    }
    let field_dimensions = ctx.field_dimensions_cache.get_nearest(image_time)?;

    let parameters = ctx.parameters.snapshot().typed().clone();
    let camera_matrix = camera_matrix.inner.clone();
    let pose_hint = pose_hint_at(image_time, &parameters, ctx.localization_cache);

    Some(PreparedDetectionFrame {
        image_time,
        objects: objects.map(|item| item.inner).unwrap_or_default(),
        robot_to_camera: robot_to_camera(&camera_matrix),
        camera_matrix,
        field_dimensions: *field_dimensions.as_ref(),
        pose_hint,
        parameters,
        include_debug: ctx.global_localization_publisher.has_subscribers(),
    })
}

fn pose_hint_at(
    image_time: Time,
    parameters: &FieldMarkAssociationParameters,
    localization_cache: &Cache<TimeWrapper<Option<AssociationPoseHint>>>,
) -> Option<Isometry3<Robot, Field>> {
    localization_cache
        .get_nearest_with_stamp(image_time)
        .and_then(|(stamp, localization)| {
            if time_distance(stamp, image_time) > parameters.pose_hint.max_pose_age {
                return None;
            }
            localization.inner.as_ref().map(|hint| hint.robot_to_field)
        })
}

fn associate_detection_frame(
    mut state: FieldMarkAssociationState,
    frame: PreparedDetectionFrame,
) -> Result<(FieldMarkAssociationState, GlobalVisualLocalization)> {
    let visual_features = crate::find_detected_visual_features(&frame.objects);
    if visual_features.supported_feature_count() == 0 {
        return Ok((
            state,
            GlobalVisualLocalization {
                debug: None,
                accepted_global_pose: None,
                associations: Vec::new(),
            },
        ));
    }

    let localization = state.associate_visual_features_with_debug(
        &visual_features,
        &frame.camera_matrix,
        &frame.field_dimensions,
        frame.pose_hint,
        &frame.parameters,
        frame.include_debug,
    );
    Ok((state, localization))
}

async fn publish_localization_frame(
    associations_publisher: &Publisher<TimeWrapper<VisualLocalizationFrame>>,
    global_localization_publisher: &Publisher<Option<GlobalLocalizationDebug>>,
    processed_frame: ProcessedDetectionFrame,
) -> Result<()> {
    let debug = processed_frame.localization.debug.clone();
    global_localization_publisher
        .publish_if_subscribed(|| ready(debug))
        .await?;
    associations_publisher
        .publish(&TimeWrapper {
            time: processed_frame.image_time,
            inner: VisualLocalizationFrame {
                robot_to_camera: processed_frame.robot_to_camera,
                associations: processed_frame.localization.associations,
                backend_reset: processed_frame.localization.accepted_global_pose,
            },
        })
        .await?;
    Ok(())
}

pub(crate) fn association_is_damping(primary_state_cache: &Cache<PrimaryState>) -> bool {
    primary_state_cache
        .get_latest()
        .is_none_or(|state| *state == PrimaryState::Damping)
}

fn camera_matrix_is_fresh(camera_matrix: &TimeWrapper<CameraMatrix>, time: Time) -> bool {
    time_distance(camera_matrix.time, time) <= MAX_CAMERA_MATRIX_TIME_DISTANCE
}

fn time_distance(a: Time, b: Time) -> Duration {
    Duration::from_nanos(a.as_nanos().abs_diff(b.as_nanos()))
}
