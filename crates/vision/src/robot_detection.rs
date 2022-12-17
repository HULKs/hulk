use std::ops::Range;

use color_eyre::Result;
use context_attribute::context;
use filtering::statistics::{mean, standard_deviation};
use framework::{AdditionalOutput, MainOutput};
use nalgebra::{distance, point};
use types::{
    Ball, CameraMatrix, ClusterCone, DetectedRobots, EdgeType, FieldDimensions, FilteredSegments,
    LineData, ScoredCluster, ScoredClusterPoint,
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

    pub amount_of_segments_factor:
        Parameter<f32, "robot_detection.$cycler_instance.amount_of_segments_factor">,
    pub amount_score_exponent:
        Parameter<f32, "robot_detection.$cycler_instance.amount_score_exponent">,
    pub cluster_cone_radius: Parameter<f32, "robot_detection.$cycler_instance.cluster_cone_radius">,
    pub cluster_distance_score_range:
        Parameter<Range<f32>, "robot_detection.$cycler_instance.cluster_distance_score_range">,
    pub detection_box_width: Parameter<f32, "robot_detection.$cycler_instance.detection_box_width">,
    pub enable: Parameter<bool, "robot_detection.$cycler_instance.enable">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub ignore_ball_segments:
        Parameter<bool, "robot_detection.$cycler_instance.ignore_ball_segments">,
    pub ignore_line_segments:
        Parameter<bool, "robot_detection.$cycler_instance.ignore_line_segments">,
    pub luminance_score_exponent:
        Parameter<f32, "robot_detection.$cycler_instance.luminance_score_exponent">,
    pub maximum_cluster_distance:
        Parameter<f32, "robot_detection.$cycler_instance.maximum_cluster_distance">,
    pub minimum_cluster_score:
        Parameter<f32, "robot_detection.$cycler_instance.minimum_cluster_score">,
    pub minimum_consecutive_segments:
        Parameter<usize, "robot_detection.$cycler_instance.minimum_consecutive_segments">,

    pub balls: RequiredInput<Option<Vec<Ball>>, "balls?">,
    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub filtered_segments: Input<FilteredSegments, "filtered_segments">,
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

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if !*context.enable {
            return Ok(MainOutputs::default());
        }

        let mut scored_cluster_points_in_pixel = extract_segment_cluster_points(
            context.filtered_segments,
            *context.minimum_consecutive_segments,
            *context.amount_of_segments_factor,
            *context.ignore_ball_segments,
            *context.ignore_line_segments,
            context.balls,
            context.line_data,
        );
        scored_cluster_points_in_pixel.sort_unstable_by(|left_point, right_point| {
            right_point.point.y.total_cmp(&left_point.point.y)
        });
        context
            .cluster_points_in_pixel
            .fill_on_subscription(|| scored_cluster_points_in_pixel.clone());

        let scored_cluster_points_in_ground =
            project_to_ground(scored_cluster_points_in_pixel, context.camera_matrix);

        let clustered_cluster_points_in_ground = cluster_scored_cluster_points(
            scored_cluster_points_in_ground,
            *context.maximum_cluster_distance,
            context.cluster_distance_score_range,
            *context.amount_score_exponent,
            *context.luminance_score_exponent,
        );
        context
            .clustered_cluster_points_in_ground
            .fill_on_subscription(|| clustered_cluster_points_in_ground.clone());

        let clusters_in_ground =
            map_clustered_cluster_points_to_scored_clusters(clustered_cluster_points_in_ground);
        let clusters_in_ground =
            filter_clusters_via_scores(clusters_in_ground, *context.minimum_cluster_score);
        let (clusters_in_ground, cluster_cones) =
            filter_clusters_via_cones(clusters_in_ground, *context.cluster_cone_radius);
        context.cluster_cones.fill_on_subscription(|| cluster_cones);

        Ok(MainOutputs {
            detected_robots: Some(DetectedRobots {
                robot_positions: clusters_in_ground,
            })
            .into(),
        })
    }
}

fn extract_segment_cluster_points(
    filtered_segments: &FilteredSegments,
    minimum_consecutive_segments: usize,
    amount_of_segments_factor: f32,
    ignore_line_segments: bool,
    ignore_ball_segments: bool,
    balls: &[Ball],
    line_data: &LineData,
) -> Vec<ScoredClusterPoint> {
    let mut cluster_points = vec![];
    for scan_line in filtered_segments.scan_grid.vertical_scan_lines.iter() {
        let mut segments = scan_line
            .segments
            .iter()
            .filter(|segment| {
                !ignore_line_segments
                    || !line_data
                        .used_vertical_filtered_segments
                        .contains(&point![scan_line.position, segment.start])
            })
            .filter(|segment| {
                !ignore_ball_segments
                    || !balls.iter().any(|ball| {
                        ball.image_location.contains(point![
                            (2 * scan_line.position) as f32,
                            segment.center() as f32
                        ])
                    })
            })
            .peekable();
        let first_segment = segments.peek();
        let mut clusters = match first_segment {
            Some(&first_segment) => {
                segments.fold(vec![vec![first_segment]], |mut clusters, segment| {
                    let last_cluster = clusters.last_mut().unwrap();
                    let belongs_to_previous_cluster =
                        segment.start == last_cluster.last().unwrap().end;
                    if belongs_to_previous_cluster {
                        last_cluster.push(segment);
                    } else {
                        let previous_cluster_too_short =
                            last_cluster.len() < minimum_consecutive_segments;
                        if previous_cluster_too_short {
                            *last_cluster = vec![segment];
                        } else {
                            clusters.push(vec![segment]);
                        }
                    }
                    clusters
                })
            }
            None => vec![],
        };
        let last_cluster_too_short = clusters.last().map_or(false, |last_cluster| {
            last_cluster.len() < minimum_consecutive_segments
        });
        if last_cluster_too_short {
            clusters.pop();
        }
        let last_cluster_reaches_border = clusters.last().map_or(false, |last_cluster| {
            let edge_type = last_cluster.last().unwrap().end_edge_type;
            edge_type == EdgeType::ImageBorder || edge_type == EdgeType::LimbBorder
        });
        if last_cluster_reaches_border {
            clusters.pop();
        }

        if let Some(last_cluster) = clusters.last() {
            let amount_of_segments = last_cluster.len();
            let luminances: Vec<_> = last_cluster
                .iter()
                .map(|segment| segment.color.y as f32)
                .collect();
            let luminance_standard_deviation = standard_deviation(&luminances, mean(&luminances));
            let amount_score = 1.0
                - (-((amount_of_segments - minimum_consecutive_segments) as f32)
                    * amount_of_segments_factor)
                    .exp();
            let luminance_score = luminance_standard_deviation / 128.0;
            cluster_points.push(ScoredClusterPoint {
                point: point![
                    scan_line.position as f32,
                    last_cluster.last().unwrap().end as f32
                ],
                amount_score,
                luminance_score,
            });
        }
    }
    cluster_points
}

fn project_to_ground(
    scored_cluster_points_in_pixel: Vec<ScoredClusterPoint>,
    camera_matrix: &CameraMatrix,
) -> Vec<ScoredClusterPoint> {
    scored_cluster_points_in_pixel
        .into_iter()
        .filter_map(|point_in_pixel| {
            camera_matrix
                .pixel_to_ground(&point![
                    point_in_pixel.point.x * 2.0,
                    point_in_pixel.point.y
                ])
                .ok()
                .map(|point_in_ground| ScoredClusterPoint {
                    point: point_in_ground,
                    amount_score: point_in_pixel.amount_score,
                    luminance_score: point_in_pixel.luminance_score,
                })
        })
        .collect()
}

fn cluster_scored_cluster_points(
    cluster_points: Vec<ScoredClusterPoint>,
    maximum_cluster_distance: f32,
    cluster_distance_score_range: &Range<f32>,
    amount_score_exponent: f32,
    luminance_score_exponent: f32,
) -> Vec<Vec<ScoredClusterPoint>> {
    let distance_score_offset = cluster_distance_score_range.start;
    let distance_score_scale =
        cluster_distance_score_range.end - cluster_distance_score_range.start;

    let mut clusters: Vec<Vec<ScoredClusterPoint>> = vec![];
    for point in cluster_points {
        let nearest_cluster = clusters
            .iter_mut()
            .map(|cluster_points| {
                let adjusted_distance = cluster_points
                    .iter()
                    .map(|cluster_point| {
                        let score = (point.amount_score.powf(amount_score_exponent)
                            * point.luminance_score.powf(luminance_score_exponent))
                            * distance_score_scale
                            + distance_score_offset;
                        distance(&cluster_point.point, &point.point) / score
                    })
                    .min_by(|left, right| left.total_cmp(right))
                    .expect("Unexpected empty cluster");
                (cluster_points, adjusted_distance)
            })
            .filter(|(_cluster, adjusted_distance)| *adjusted_distance <= maximum_cluster_distance)
            .min_by(
                |(_left_cluster, left_adjusted_distance),
                 (_right_cluster, right_adjusted_distance)| {
                    left_adjusted_distance.total_cmp(right_adjusted_distance)
                },
            );
        match nearest_cluster {
            Some((nearest_cluster, _adjusted_distance_to_nearest_cluster)) => {
                nearest_cluster.push(point)
            }
            None => clusters.push(vec![point]),
        }
    }
    clusters
}

fn map_clustered_cluster_points_to_scored_clusters(
    clustered_cluster_points: Vec<Vec<ScoredClusterPoint>>,
) -> Vec<ScoredCluster> {
    clustered_cluster_points
        .into_iter()
        .map(|cluster_points| {
            let (mut xs, mut ys): (Vec<_>, Vec<_>) = cluster_points
                .iter()
                .map(|point| (point.point.x, point.point.y))
                .unzip();
            xs.sort_by(|left, right| left.total_cmp(right));
            ys.sort_by(|left, right| left.total_cmp(right));
            let median = point![xs[xs.len() / 2], ys[ys.len() / 2]];
            let score = cluster_points
                .iter()
                .map(|point| point.amount_score * point.luminance_score)
                .sum();
            ScoredCluster {
                center: median,
                score,
            }
        })
        .collect()
}

fn filter_clusters_via_scores(
    clusters: Vec<ScoredCluster>,
    minimum_cluster_score: f32,
) -> Vec<ScoredCluster> {
    clusters
        .into_iter()
        .filter(|cluster| cluster.score >= minimum_cluster_score)
        .collect()
}

fn filter_clusters_via_cones(
    clusters: Vec<ScoredCluster>,
    cluster_cone_radius: f32,
) -> (Vec<ScoredCluster>, Vec<ClusterCone>) {
    clusters
        .into_iter()
        .fold((vec![], vec![]), |(mut clusters, mut cones), cluster| {
            let cone = ClusterCone::from_cluster(&cluster, cluster_cone_radius);
            if !cones
                .iter()
                .any(|existing_cone| existing_cone.intersects_with(&cone))
            {
                clusters.push(cluster);
                cones.push(cone);
            }
            (clusters, cones)
        })
}
