use std::ops::Range;

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, Parameter, RequiredInput};
use types::{
    Ball, CameraMatrix, ClusterCone, DetectedRobots, FieldDimensions, FilteredSegments, LineData,
    ScoredClusterPoint,
};

pub struct RobotDetection {}

#[context]
pub struct NewContext {
    pub amount_of_segments_factor:
        Parameter<f32, "robot_detection/$cycler_instance/amount_of_segments_factor">,
    pub amount_score_exponent:
        Parameter<f32, "robot_detection/$cycler_instance/amount_score_exponent">,
    pub cluster_cone_radius: Parameter<f32, "robot_detection/$cycler_instance/cluster_cone_radius">,
    pub cluster_distance_score_range:
        Parameter<Range<f32>, "robot_detection/$cycler_instance/cluster_distance_score_range">,
    pub detection_box_width: Parameter<f32, "robot_detection/$cycler_instance/detection_box_width">,
    pub enable: Parameter<bool, "robot_detection/$cycler_instance/enable">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub ignore_ball_segments:
        Parameter<bool, "robot_detection/$cycler_instance/ignore_ball_segments">,
    pub ignore_line_segments:
        Parameter<bool, "robot_detection/$cycler_instance/ignore_line_segments">,
    pub luminance_score_exponent:
        Parameter<f32, "robot_detection/$cycler_instance/luminance_score_exponent">,
    pub maximum_cluster_distance:
        Parameter<f32, "robot_detection/$cycler_instance/maximum_cluster_distance">,
    pub minimum_cluster_score:
        Parameter<f32, "robot_detection/$cycler_instance/minimum_cluster_score">,
    pub minimum_consecutive_segments:
        Parameter<usize, "robot_detection/$cycler_instance/minimum_consecutive_segments">,
}

#[context]
pub struct CycleContext {
    pub cluster_cones: AdditionalOutput<Vec<ClusterCone>, "robot_detection/cluster_cones">,
    pub cluster_points_in_pixel:
        AdditionalOutput<Vec<ScoredClusterPoint>, "robot_detection/cluster_points_in_pixel">,
    pub clustered_cluster_points_in_ground: AdditionalOutput<
        Vec<Vec<ScoredClusterPoint>>,
        "robot_detection/clustered_cluster_points_in_ground",
    >,

    pub amount_of_segments_factor:
        Parameter<f32, "robot_detection/$cycler_instance/amount_of_segments_factor">,
    pub amount_score_exponent:
        Parameter<f32, "robot_detection/$cycler_instance/amount_score_exponent">,
    pub cluster_cone_radius: Parameter<f32, "robot_detection/$cycler_instance/cluster_cone_radius">,
    pub cluster_distance_score_range:
        Parameter<Range<f32>, "robot_detection/$cycler_instance/cluster_distance_score_range">,
    pub detection_box_width: Parameter<f32, "robot_detection/$cycler_instance/detection_box_width">,
    pub enable: Parameter<bool, "robot_detection/$cycler_instance/enable">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub ignore_ball_segments:
        Parameter<bool, "robot_detection/$cycler_instance/ignore_ball_segments">,
    pub ignore_line_segments:
        Parameter<bool, "robot_detection/$cycler_instance/ignore_line_segments">,
    pub luminance_score_exponent:
        Parameter<f32, "robot_detection/$cycler_instance/luminance_score_exponent">,
    pub maximum_cluster_distance:
        Parameter<f32, "robot_detection/$cycler_instance/maximum_cluster_distance">,
    pub minimum_cluster_score:
        Parameter<f32, "robot_detection/$cycler_instance/minimum_cluster_score">,
    pub minimum_consecutive_segments:
        Parameter<usize, "robot_detection/$cycler_instance/minimum_consecutive_segments">,

    pub balls: RequiredInput<Option<Vec<Ball>>, "balls?">,
    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub filtered_segments: RequiredInput<Option<FilteredSegments>, "filtered_segments?">,
    pub line_data: RequiredInput<Option<LineData>, "line_data?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_robots: MainOutput<Option<DetectedRobots>>,
}

impl RobotDetection {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
