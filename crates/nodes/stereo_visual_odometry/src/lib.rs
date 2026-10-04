mod feature_extractor;
mod odometry;
mod parameters;
mod pipeline;
mod pose_refinement;
mod tracking;
mod triangulator;

pub use odometry::{OdometryDiagnostics, PoseEvaluationDiagnostics};
pub use parameters::{
    StereoVisualOdometryParameters, StereoVisualOdometryPoseEstimationParameters,
};
pub use pipeline::VisualOdometryPipeline;
pub use tracking::{FrameOutput, TrackingOutcome};

use std::{
    boxed::Box,
    future::{Future, ready},
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::Result;
use nalgebra as na;

use coordinate_systems::Camera;
use linear_algebra::Point3;
use ros_z::prelude::*;
use ros_z::qos::QosDurability;
use types::{
    stereo_camera_info::StereoCameraInfo, stereo_image_pair::StereoImagePair,
    visual_odometry::VisualOdometer,
};

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
        .subscriber::<StereoImagePair>("inputs/stereo_image_pair")
        .build()
        .await?;

    // This measures the full pipeline, including triangulation and PnP/LM.
    let processing_duration_pub = node
        .publisher::<Duration>("debug/visual_odometry/processing_duration")
        .build()
        .await?;

    let debug_odometry_pub = node
        .publisher::<Option<na::Isometry3<f32>>>(
            "debug/visual_odometry/previous_left_camera_to_current_left_camera",
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

    loop {
        parameters_receiver
            .wait_for(|parameters| parameters.typed().enable)
            .await?;

        let stereo_image_pair = stereo_image_pair_sub.recv().await?;
        let parameters = node_parameters.snapshot();
        let parameters = parameters.typed();

        let (output, duration) = tokio::task::block_in_place(|| {
            let start_time = Instant::now();
            let odometry =
                pipeline.process(&stereo_image_pair, &parameters.pose_estimation_parameters);
            (odometry, start_time.elapsed())
        });

        if let TrackingOutcome::Reset { error } = &output.outcome {
            if let Some(error) = error {
                tracing::warn!(
                    ?error,
                    "visual odometry frame processing failed. Reset tracking"
                );
            } else {
                tracing::debug!("visual odometry estimate failed. Odometer epoch reset");
            }
        }

        debug_odometry_pub
            .publish_if_subscribed(|| ready(output.previous_to_current()))
            .await?;
        odometer_pub.publish(&output.odometer).await?;
        if !matches!(output.outcome, TrackingOutcome::Reset { .. })
            && triangulated_features_pub.has_subscribers()
        {
            let triangulated_features = pipeline.triangulated_features();
            triangulated_features_pub
                .publish_if_subscribed(|| ready(triangulated_features))
                .await?;
        }
        processing_duration_pub
            .publish_if_subscribed(|| ready(duration))
            .await?;
    }
}
