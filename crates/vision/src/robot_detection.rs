use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, Parameter, RequiredInput};

pub struct RobotDetection {}

#[context]
pub struct NewContext {
    pub amount_of_segments_factor:
        Parameter<f32, "$this_cycler/robot_detection/amount_of_segments_factor">,
    pub amount_score_exponent: Parameter<f32, "$this_cycler/robot_detection/amount_score_exponent">,
    pub cluster_cone_radius: Parameter<f32, "$this_cycler/robot_detection/cluster_cone_radius">,
    pub cluster_distance_score_range:
        Parameter<Range<f32>, "$this_cycler/robot_detection/cluster_distance_score_range">,
    pub detection_box_width: Parameter<f32, "$this_cycler/robot_detection/detection_box_width">,
    pub enable: Parameter<bool, "$this_cycler/robot_detection/enable">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub ignore_ball_segments: Parameter<bool, "$this_cycler/robot_detection/ignore_ball_segments">,
    pub ignore_line_segments: Parameter<bool, "$this_cycler/robot_detection/ignore_line_segments">,
    pub luminance_score_exponent:
        Parameter<f32, "$this_cycler/robot_detection/luminance_score_exponent">,
    pub maximum_cluster_distance:
        Parameter<f32, "$this_cycler/robot_detection/maximum_cluster_distance">,
    pub minimum_cluster_score: Parameter<f32, "$this_cycler/robot_detection/minimum_cluster_score">,
    pub minimum_consecutive_segments:
        Parameter<usize, "$this_cycler/robot_detection/minimum_consecutive_segments">,
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
        Parameter<f32, "$this_cycler/robot_detection/amount_of_segments_factor">,
    pub amount_score_exponent: Parameter<f32, "$this_cycler/robot_detection/amount_score_exponent">,
    pub cluster_cone_radius: Parameter<f32, "$this_cycler/robot_detection/cluster_cone_radius">,
    pub cluster_distance_score_range:
        Parameter<Range<f32>, "$this_cycler/robot_detection/cluster_distance_score_range">,
    pub detection_box_width: Parameter<f32, "$this_cycler/robot_detection/detection_box_width">,
    pub enable: Parameter<bool, "$this_cycler/robot_detection/enable">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub ignore_ball_segments: Parameter<bool, "$this_cycler/robot_detection/ignore_ball_segments">,
    pub ignore_line_segments: Parameter<bool, "$this_cycler/robot_detection/ignore_line_segments">,
    pub luminance_score_exponent:
        Parameter<f32, "$this_cycler/robot_detection/luminance_score_exponent">,
    pub maximum_cluster_distance:
        Parameter<f32, "$this_cycler/robot_detection/maximum_cluster_distance">,
    pub minimum_cluster_score: Parameter<f32, "$this_cycler/robot_detection/minimum_cluster_score">,
    pub minimum_consecutive_segments:
        Parameter<usize, "$this_cycler/robot_detection/minimum_consecutive_segments">,

    pub balls: RequiredInput<Vec<Ball>, "balls">,
    pub camera_matrix: RequiredInput<CameraMatrix, "camera_matrix">,
    pub filtered_segments: RequiredInput<FilteredSegments, "filtered_segments">,
    pub line_data: RequiredInput<LineData, "line_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_robots: MainOutput<DetectedRobots>,
}

impl RobotDetection {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
