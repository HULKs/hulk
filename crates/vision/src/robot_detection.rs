use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{
    Ball, CameraMatrix, ClusterCone, DetectedRobots, FilteredSegments, LineData, ScoredClusterPoint,
};

pub struct RobotDetection {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub cluster_cones: AdditionalOutput<Vec<ClusterCone>, "robot_detection.cluster_cones">,
    pub cluster_points_in_pixel:
        AdditionalOutput<Vec<ScoredClusterPoint>, "robot_detection.cluster_points_in_pixel">,
    pub clustered_cluster_points_in_ground: AdditionalOutput<
        Vec<Vec<ScoredClusterPoint>>,
        "robot_detection.clustered_cluster_points_in_ground",
    >,

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
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
