mod feature_extractor;
mod odometry;
pub mod parameters;
pub mod pipeline;
mod pose_refinement;
mod triangulator;

pub use odometry::OdometryDiagnostics;
pub use pipeline::VisualOdometryPipeline;

use std::{
    boxed::Box,
    future::{Future, ready},
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::{Result, eyre::WrapErr};
use coordinate_systems::Camera;
use linear_algebra::Point3;
use nalgebra as na;

use ros_z::prelude::*;
use ros_z::qos::QosDurability;
use types::{
    stereo_camera_info::StereoCameraInfo,
    stereo_image_pair::StereoImagePair,
    time_wrapper::TimeWrapper,
    visual_odometry::{VisualOdometer, VisualOdometryDelta},
};

use crate::parameters::StereoVisualOdometryParameters;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("stereo_visual_odometry").build().await?;
    let node_parameters =
        node.bind_parameter_as::<StereoVisualOdometryParameters>("stereo_visual_odometry")?;
    node_parameters.add_validation_hook(StereoVisualOdometryParameters::validate)?;
    let mut parameters_receiver = node_parameters.subscribe();

    let stereo_camera_info_sub = node
        .subscriber::<StereoCameraInfo>("inputs/stereo_camera_info")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let stereo_image_pair_sub = node
        .subscriber::<TimeWrapper<StereoImagePair>>("inputs/stereo_image_pair")
        .build()
        .await?;

    let feature_duration_pub = node
        .publisher::<Duration>("debug/visual_odometry/feature_extraction_duration")
        .build()
        .await?;

    let debug_odometry_pub = node
        .publisher::<Option<na::Isometry3<f32>>>(
            "debug/visual_odometry/previous_left_camera_to_current_left_camera",
        )
        .build()
        .await?;

    let delta_odometry_pub = node
        .publisher::<VisualOdometryDelta>(
            "visual_odometry/current_left_camera_to_previous_left_camera",
        )
        .build()
        .await?;

    let odometer_pub = node
        .publisher::<VisualOdometer>("visual_odometry/current_left_camera_to_visual_odometer")
        .build()
        .await?;

    // Caution: We don't yet differentiate between left and right camera frames.
    let triangulated_features_pub = node
        .publisher::<Vec<Point3<Camera>>>("debug/visual_odometry/triangulated_features")
        .build()
        .await?;

    let stereo_camera_info = stereo_camera_info_sub.recv().await?;
    let parameters = node_parameters.snapshot();
    let mut pipeline =
        VisualOdometryPipeline::new(&parameters.typed().neural_network, stereo_camera_info)?;
    let mut previous_image_time: Option<ros_z::time::Time> = None;
    let mut odometer_epoch = 0;

    loop {
        parameters_receiver
            .wait_for(|parameters| parameters.typed().enable)
            .await?;

        let stereo_image_pair = stereo_image_pair_sub.recv().await?;
        let current_image_time = stereo_image_pair.time;
        let stereo_image_pair = stereo_image_pair.inner;
        let parameters = node_parameters.snapshot();
        let parameters = parameters.typed();

        let had_previous_image = previous_image_time.is_some();
        let pose_estimation_parameters = parameters.pose_estimation_parameters.clone();
        let (returned_pipeline, odometry_result, duration) =
            tokio::task::spawn_blocking(move || {
                let start_time = Instant::now();
                let odometry = pipeline.process(&stereo_image_pair, &pose_estimation_parameters);
                (pipeline, odometry, start_time.elapsed())
            })
            .await
            .wrap_err("visual odometry task failed")?;
        pipeline = returned_pipeline;

        let mut process_failed = false;
        let odometry = match odometry_result {
            Ok(odometry) => odometry,
            Err(error) => {
                tracing::warn!(
                    ?error,
                    "visual odometry frame processing failed; resetting tracking"
                );
                process_failed = true;
                None
            }
        };

        debug_odometry_pub
            .publish_if_subscribed(|| ready(odometry))
            .await?;
        if process_failed || (had_previous_image && odometry.is_none()) {
            tracing::debug!("visual odometry estimate failed; resetting odometer epoch");
            odometer_epoch += 1;
            previous_image_time = None;
            pipeline.reset_tracking();
            odometer_pub
                .publish(&VisualOdometer {
                    time: current_image_time,
                    epoch: odometer_epoch,
                    current_left_camera_to_visual_odometer: pipeline
                        .current_left_camera_to_visual_odometer(),
                })
                .await?;
            feature_duration_pub
                .publish_if_subscribed(|| ready(duration))
                .await?;
            continue;
        }
        if let (Some(previous_time), Some(previous_left_camera_to_current_left_camera)) =
            (previous_image_time, odometry.as_ref())
        {
            delta_odometry_pub
                .publish(&VisualOdometryDelta {
                    previous_time,
                    current_time: current_image_time,
                    current_left_camera_to_previous_left_camera:
                        previous_left_camera_to_current_left_camera.inverse(),
                })
                .await?;
        }
        previous_image_time = Some(current_image_time);
        odometer_pub
            .publish(&VisualOdometer {
                time: current_image_time,
                epoch: odometer_epoch,
                current_left_camera_to_visual_odometer: pipeline
                    .current_left_camera_to_visual_odometer(),
            })
            .await?;
        if triangulated_features_pub.has_subscribers() {
            let triangulated_features = pipeline.triangulated_features();
            triangulated_features_pub
                .publish_if_subscribed(|| ready(triangulated_features))
                .await?;
        }
        feature_duration_pub
            .publish_if_subscribed(|| ready(duration))
            .await?;
    }
}
