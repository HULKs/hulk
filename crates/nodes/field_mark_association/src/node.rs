use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use projection::camera_matrix::CameraMatrix;
use ros_z::{
    context::Context,
    parameter::NodeParametersExt,
    qos::{QosDurability, QosProfile},
};
use ros_z_streams::CreateFutureMapBuilder;
use types::{
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
    visual_localization::{
        ASSOCIATION_POSE_HINT_TOPIC, AssociationPoseHint, GLOBAL_LOCALIZATION_DEBUG_TOPIC,
        GlobalLocalizationDebug, VISUAL_LOCALIZATION_TOPIC, VisualLocalizationFrame,
    },
};

use crate::{
    FieldMarkAssociationState,
    frame_processing::{DetectionProcessingContext, process_detected_objects},
    parameters::FieldMarkAssociationParameters,
};

const DETECTED_OBJECTS_SAFETY_LAG: Duration = Duration::from_millis(50);

/// Starts the field-mark association node and erases the concrete future type for node runners.
pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("field_mark_association").build().await?;
    let parameters =
        node.bind_parameter_as::<FieldMarkAssociationParameters>("field_mark_association")?;
    parameters.add_validation_hook(FieldMarkAssociationParameters::validate)?;

    let camera_matrix_cache = node
        .subscriber::<TimeWrapper<CameraMatrix>>("camera_matrix")
        .cache(128)
        .with_stamp(|message| message.time)
        .build()
        .await?;

    let field_dimensions_cache = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;

    let localization_cache = node
        .subscriber::<TimeWrapper<Option<AssociationPoseHint>>>(ASSOCIATION_POSE_HINT_TOPIC)
        .cache(128)
        .with_stamp(|message| message.time)
        .build()
        .await?;

    let primary_state_cache = node
        .subscriber::<PrimaryState>("primary_state")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;

    let primary_state_subscriber = node
        .subscriber::<PrimaryState>("primary_state")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let mut detected_objects = node
        .create_future_map_builder()
        .create_future_subscriber::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>(
            "detected_objects",
            DETECTED_OBJECTS_SAFETY_LAG,
        )
        .await?
        .build();

    let associations_publisher = node
        .publisher::<TimeWrapper<VisualLocalizationFrame>>(VISUAL_LOCALIZATION_TOPIC)
        .build()
        .await?;
    let global_localization_publisher = node
        .publisher::<Option<GlobalLocalizationDebug>>(GLOBAL_LOCALIZATION_DEBUG_TOPIC)
        .build()
        .await?;
    let mut association_state = FieldMarkAssociationState::default();

    loop {
        tokio::select! {
            primary_state = primary_state_subscriber.recv() => {
                if primary_state? == PrimaryState::Damping {
                    association_state.reset_for_damping();
                }
            }
            item = detected_objects.recv() => {
                let item = item?;
                process_detected_objects(
                    item,
                    DetectionProcessingContext {
                        parameters: &parameters,
                        camera_matrix_cache: &camera_matrix_cache,
                        field_dimensions_cache: &field_dimensions_cache,
                        localization_cache: &localization_cache,
                        primary_state_cache: &primary_state_cache,
                        association_state: &mut association_state,
                        associations_publisher: &associations_publisher,
                        global_localization_publisher: &global_localization_publisher,
                    },
                ).await?;
            }
        }
    }
}
