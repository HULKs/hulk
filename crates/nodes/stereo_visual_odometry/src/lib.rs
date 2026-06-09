mod feature_extractor;
mod odometry;
mod triangulator;

use std::{
    boxed::Box,
    future::Future,
    path::Path,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::Result;
use nalgebra as na;

use ros_z::prelude::*;
use ros_z::qos::QosDurability;
use types::{
    parameters::StereoVisualOdometryParameters, stereo_camera_info::StereoCameraInfo,
    stereo_image_pair::StereoImagePair, time_wrapper::TimeWrapper,
    visual_odometry::TriangulatedFeature,
};

use crate::{
    feature_extractor::{FeatureExtractor, KEYPOINTS, PreviousFeatureState},
    odometry::{OdometryScratch, PreviousFrame, estimate_previous_to_current},
    triangulator::StereoTriangulator,
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("stereo_visual_odometry").build().await?;
    let node_parameters =
        node.bind_parameter_as::<StereoVisualOdometryParameters>("stereo_visual_odometry")?;
    let mut parameters_receiver = node_parameters.subscribe();

    let stereo_camera_info_sub = node
        .subscriber::<StereoCameraInfo>("inputs/stereo_camera_info")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let stereo_image_pair_sub = node
        .subscriber::<TimeWrapper<StereoImagePair>>("inputs/stereo_image_pair")?
        .build()
        .await?;

    let feature_duration_pub = node
        .publisher::<Duration>("visual_odometry/feature_extraction_duration")?
        .build()
        .await?;
    let odometry_pub = node
        .publisher::<Option<na::Isometry3<f32>>>(
            "visual_odometry/previous_left_camera_to_current_left_camera",
        )?
        .build()
        .await?;
    let odometry_state_pub = node
        .publisher::<na::Isometry3<f32>>("visual_odometry/current_left_camera_to_visual_odometer")?
        .build()
        .await?;
    let triangulated_features_pub = node
        .publisher::<Vec<TriangulatedFeature>>("visual_odometry/triangulated_features")?
        .build()
        .await?;

    let stereo_camera_info = stereo_camera_info_sub.recv().await?;
    let parameters = node_parameters.snapshot();
    let mut pipeline = VisualOdometryPipeline::new(
        parameters
            .typed()
            .neural_networks_folder
            .join(&parameters.typed().model_name),
        stereo_camera_info,
    )?;

    loop {
        parameters_receiver
            .wait_for(|parameters| parameters.typed().enable)
            .await?;

        let stereo_image_pair = stereo_image_pair_sub.recv().await?.inner;

        let start_time = Instant::now();
        let odometry = pipeline.process(stereo_image_pair)?;
        let duration = start_time.elapsed();

        odometry_pub.publish(&odometry).await?;
        odometry_state_pub
            .publish(&pipeline.current_left_camera_to_visual_odometer())
            .await?;
        triangulated_features_pub
            .publish(&pipeline.triangulated_features())
            .await?;
        feature_duration_pub.publish(&duration).await?;
    }
}

struct VisualOdometryPipeline {
    feature_extractor: FeatureExtractor,
    triangulator: StereoTriangulator,
    previous_features: PreviousFeatureState,
    previous_frame: Option<PreviousFrame>,
    current_points: Vec<crate::triangulator::StereoPoint>,
    current_left_camera_to_visual_odometer: na::Isometry3<f32>,
    odometry_scratch: OdometryScratch,
}

impl VisualOdometryPipeline {
    fn new(model_path: impl AsRef<Path>, stereo_camera_info: StereoCameraInfo) -> Result<Self> {
        Ok(Self {
            feature_extractor: FeatureExtractor::new(model_path)?,
            triangulator: StereoTriangulator::new(
                &stereo_camera_info.left,
                &stereo_camera_info.right,
            )?,
            previous_features: PreviousFeatureState::new(),
            previous_frame: None,
            current_points: Vec::with_capacity(KEYPOINTS),
            current_left_camera_to_visual_odometer: na::Isometry3::identity(),
            odometry_scratch: OdometryScratch::new(),
        })
    }

    fn process(
        &mut self,
        stereo_image_pair: StereoImagePair,
    ) -> Result<Option<na::Isometry3<f32>>> {
        let odometry = {
            let features = self
                .feature_extractor
                .extract(&stereo_image_pair, &self.previous_features)?;
            let current_left = features.current_left()?;
            let current_right = features.current_right()?;
            let stereo_matches = features.stereo_matches()?;

            println!("{} stereo matches", stereo_matches.left_to_right().count());

            self.triangulator.triangulate_into(
                current_left,
                current_right,
                stereo_matches,
                &mut self.current_points,
            );

            println!("{} triangulations", self.current_points.len());

            if let Some(previous_frame) = self.previous_frame.as_ref() {
                let temporal_matches = features.temporal_matches()?;
                println!(
                    "{} temporal matches",
                    temporal_matches.left_to_right().count()
                );
                let odometry = estimate_previous_to_current(
                    previous_frame,
                    &current_left,
                    &temporal_matches,
                    &self.triangulator,
                    &mut self.odometry_scratch,
                );
                features.copy_current_left_to(&mut self.previous_features)?;
                odometry
            } else {
                features.copy_current_left_to(&mut self.previous_features)?;
                None
            }
        };

        if let Some(previous_frame) = self.previous_frame.as_mut() {
            previous_frame.replace_stereo_points(&self.current_points);
        } else {
            self.previous_frame = Some(PreviousFrame::from_stereo_points(&self.current_points));
        }

        if let Some(previous_to_current) = &odometry {
            self.current_left_camera_to_visual_odometer *= previous_to_current.inverse();
        }

        Ok(odometry)
    }

    fn current_left_camera_to_visual_odometer(&self) -> na::Isometry3<f32> {
        self.current_left_camera_to_visual_odometer
    }

    fn triangulated_features(&self) -> Vec<TriangulatedFeature> {
        self.current_points
            .iter()
            .map(|point| TriangulatedFeature {
                x: point.position.x,
                y: point.position.y,
                z: point.position.z,
            })
            .collect()
    }
}
