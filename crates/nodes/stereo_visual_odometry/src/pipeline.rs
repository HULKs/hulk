use std::path::Path;

use crate::{
    feature_extractor::{FeatureExtractor, NUM_KEYPOINTS, PreviousFeatureState},
    odometry::{OdometryDiagnostics, OdometryScratch, PreviousFrame, estimate_previous_to_current},
    parameters::StereoVisualOdometryPoseEstimationParameters,
    triangulator::StereoTriangulator,
};

use coordinate_systems::Camera;
use linear_algebra::{Point3, point};
use types::{stereo_camera_info::StereoCameraInfo, stereo_image_pair::StereoImagePair};

use color_eyre::{Result, eyre::Report};
use nalgebra as na;

/// Stateful stereo visual odometry pipeline.
///
/// The first processed frame initializes feature state and returns `None`. Later
/// frames return the previous-left-camera to current-left-camera transform when
/// enough temporal correspondences survive geometric checks.
pub struct VisualOdometryPipeline {
    feature_extractor: FeatureExtractor,
    triangulator: StereoTriangulator,
    previous_features: PreviousFeatureState,
    previous_frame: Option<PreviousFrame>,
    current_points: Vec<crate::triangulator::StereoPoint>,
    current_left_camera_to_visual_odometer: na::Isometry3<f32>,
    odometry_scratch: OdometryScratch,
}

impl VisualOdometryPipeline {
    /// Create a pipeline for one fixed stereo camera calibration and ONNX model.
    pub fn new(model_path: impl AsRef<Path>, stereo_camera_info: StereoCameraInfo) -> Result<Self> {
        Ok(Self {
            feature_extractor: FeatureExtractor::new(model_path)?,
            triangulator: StereoTriangulator::new(
                &stereo_camera_info.left,
                &stereo_camera_info.right,
            )?,
            previous_features: PreviousFeatureState::new(),
            previous_frame: None,
            current_points: Vec::with_capacity(NUM_KEYPOINTS),
            current_left_camera_to_visual_odometer: na::Isometry3::identity(),
            odometry_scratch: OdometryScratch::new(),
        })
    }

    /// Process one NV12 stereo frame pair.
    ///
    /// Returns `None` for the initialization frame and when pose estimation does
    /// not have enough valid correspondences.
    pub fn process(
        &mut self,
        stereo_image_pair: &StereoImagePair,
        parameters: &StereoVisualOdometryPoseEstimationParameters,
    ) -> Result<Option<na::Isometry3<f32>>> {
        parameters.validate().map_err(Report::msg)?;

        let odometry = {
            let features = self
                .feature_extractor
                .extract(stereo_image_pair, &self.previous_features)?;
            let current_left = features.current_left()?;
            let current_right = features.current_right()?;
            let stereo_matches = features.stereo_matches()?;

            self.triangulator.triangulate_into(
                current_left,
                current_right,
                stereo_matches,
                parameters.max_vertical_disparity_px,
                &mut self.current_points,
            );

            if let Some(previous_frame) = self.previous_frame.as_ref() {
                let temporal_matches = features.temporal_matches()?;
                let odometry = estimate_previous_to_current(
                    previous_frame,
                    &current_left,
                    &self.current_points,
                    &temporal_matches,
                    &self.triangulator,
                    parameters,
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

    /// Return the latest current-left-camera to visual-odometer transform.
    ///
    /// This remains identity until the first valid frame-to-frame odometry update.
    /// The transform uses the left-camera frame and does not include head-camera extrinsics.
    pub fn current_left_camera_to_visual_odometer(&self) -> na::Isometry3<f32> {
        self.current_left_camera_to_visual_odometer
    }

    pub fn latest_odometry_diagnostics(&self) -> OdometryDiagnostics {
        self.odometry_scratch.diagnostics()
    }

    pub fn reset_tracking(&mut self) {
        self.previous_features = PreviousFeatureState::new();
        self.previous_frame = None;
        self.current_points.clear();
        self.current_left_camera_to_visual_odometer = na::Isometry3::identity();
    }

    /// Return the stereo points triangulated from the most recently processed frame.
    ///
    /// Points are expressed in the current left-camera frame.
    pub fn triangulated_features(&self) -> Vec<Point3<Camera>> {
        self.current_points
            .iter()
            .map(|point| {
                point! {
                    point.position.x,
                    point.position.y,
                    point.position.z,
                }
            })
            .collect()
    }
}
