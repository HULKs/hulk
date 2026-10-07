use std::{future::Future, num::NonZeroUsize, pin::Pin, sync::Arc};

use color_eyre::{Result, eyre::OptionExt as _};
use ros_z::{
    context::Context,
    parameter::NodeParametersExt,
    qos::{QosDurability, QosHistory, QosProfile},
};
use types::{
    camera_geometry::CameraGeometry,
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
    visual_localization::{GlobalLocalizationDebug, VisualLocalizationFrame},
};

use crate::{
    frame_processing::{
        DetectionProcessingContext, keep_latest_detection, process_detected_objects,
    },
    parameters::FieldMarkAssociationParameters,
};

/// Starts the field-mark association node and erases the concrete future type for node runners.
pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("field_mark_association").build().await?;
    let parameters =
        node.bind_parameter_as::<FieldMarkAssociationParameters>("field_mark_association")?;
    let capacities = parameters.snapshot().typed.capacities;
    parameters.add_validation_hook(move |candidate| candidate.validate_update(capacities))?;

    let camera_geometry_cache = node
        .subscriber::<TimeWrapper<CameraGeometry>>("camera_geometry")
        .cache(capacities.camera_geometry)
        .with_stamp(|message| message.time)
        .build()
        .await?;

    let field_dimensions_cache = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(capacities.field_dimensions)
        .build()
        .await?;

    let estimates = node
        .subscriber::<types::localization::LocalizationEstimate>("localization/estimate")
        .cache(capacities.estimates)
        .with_stamp(|message| message.time)
        .build()
        .await?;
    let status = node
        .subscriber::<types::localization::LocalizationStatus>("localization/status")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(capacities.status)
        .build()
        .await?;

    // Consume only payloads, without joining the announcement protocol or its KeepAll queue.
    let detected_objects = node
        .subscriber::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>("detected_objects")
        .qos(QosProfile {
            history: QosHistory::KeepLast(
                NonZeroUsize::new(capacities.detections_queue)
                    .ok_or_eyre("field_mark_association.capacities.detections_queue must be > 0")?,
            ),
            ..Default::default()
        })
        .build()
        .await?;

    let associations_publisher = node
        .publisher::<TimeWrapper<VisualLocalizationFrame>>(
            "field_mark_association/visual_localization_local",
        )
        .build()
        .await?;
    let global_localization_publisher = node
        .publisher::<Option<GlobalLocalizationDebug>>("debug/global_localization")
        .build()
        .await?;
    let processing_context = DetectionProcessingContext {
        parameters: &parameters,
        camera_geometry_cache: &camera_geometry_cache,
        field_dimensions_cache: &field_dimensions_cache,
        estimates: &estimates,
        status: &status,
        attitudes: std::sync::Mutex::new(ros_z::cache::CacheInner::new(capacities.attitudes)),
        associations_publisher: Arc::new(associations_publisher),
        global_localization_publisher: Arc::new(global_localization_publisher),
        clock: node.clock(),
    };
    let imu = node
        .subscriber::<booster::ImuState>("inputs/imu_state")
        .queue_capacity(
            NonZeroUsize::new(capacities.imu_queue)
                .ok_or_eyre("field_mark_association.capacities.imu_queue must be > 0")?,
        )
        .build()
        .await?;
    let mut pending_frame = None;
    loop {
        // A completion may win select while a newer payload is already in the subscriber queue.
        // This loop is the sole receiver, so a ready queue cannot be drained by another task.
        if pending_frame.is_some() && detected_objects.is_ready() {
            keep_latest_detection(&mut pending_frame, detected_objects.recv().await?);
        }
        let objects = match pending_frame.take() {
            Some(frame) => frame,
            None => tokio::select! {
                sample = imu.recv_with_metadata() => {
                    let sample = sample?;
                    processing_context.record_attitude(sample.source_time, &sample.message);
                    continue;
                }
                objects = detected_objects.recv() => objects?,
            },
        };
        let image_time = objects.time;
        let processing = process_detected_objects(objects, &processing_context);
        tokio::pin!(processing);
        // Keep receiving even while the solver or either publisher is waiting.
        loop {
            tokio::select! {
                sample = imu.recv_with_metadata() => {
                    let sample = sample?;
                    processing_context.record_attitude(sample.source_time, &sample.message);
                }
                result = &mut processing => {
                    result?;
                    break;
                }
                objects = detected_objects.recv() => {
                    let objects = objects?;
                    if objects.time > image_time {
                        keep_latest_detection(&mut pending_frame, objects);
                    }
                }
            }
        }
    }
}
