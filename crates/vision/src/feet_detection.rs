use color_eyre::Result;
use context_attribute::context;
use filtering::{
    mean_clustering::MeanClustering,
    statistics::{mean, standard_deviation},
};
use framework::{AdditionalOutput, MainOutput};
use itertools::Itertools;
use nalgebra::{distance, point, Point2};
use projection::Projection;
use types::{
    detected_feet::{ClusterPoint, CountedCluster, DetectedFeet},
    Ball, CameraMatrix, EdgeType, FieldDimensions, FilteredSegments, LineData, ScanLine, Segment,
};

pub struct FeetDetection {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cluster_points: AdditionalOutput<Vec<ClusterPoint>, "feet_detection.cluster_points">,
    clusters_in_ground: AdditionalOutput<Vec<Point2<f32>>, "feet_detection.clusters_in_ground">,

    enable: Parameter<bool, "feet_detection.$cycler_instance.enable">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    maximum_cluster_distance:
        Parameter<f32, "feet_detection.$cycler_instance.maximum_cluster_distance">,
    minimum_consecutive_segments:
        Parameter<usize, "feet_detection.$cycler_instance.minimum_consecutive_segments">,
    minimum_luminance_standard_deviation:
        Parameter<f32, "feet_detection.$cycler_instance.minimum_luminance_standard_deviation">,
    minimum_samples_per_cluster:
        Parameter<usize, "feet_detection.$cycler_instance.minimum_samples_per_cluster">,

    balls: RequiredInput<Option<Vec<Ball>>, "balls?">,
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    filtered_segments: Input<FilteredSegments, "filtered_segments">,
    line_data: RequiredInput<Option<LineData>, "line_data?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_feet: MainOutput<DetectedFeet>,
}

impl FeetDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if !context.enable {
            return Ok(MainOutputs::default());
        }

        let cluster_points = extract_segment_cluster_points(
            context.filtered_segments,
            *context.minimum_consecutive_segments,
            *context.minimum_luminance_standard_deviation,
            context.balls,
            context.line_data,
            context.camera_matrix,
        );
        context
            .cluster_points
            .fill_if_subscribed(|| cluster_points.clone());

        let clusters_in_ground =
            cluster_scored_cluster_points(cluster_points, *context.maximum_cluster_distance);
        let clusters_in_ground: Vec<_> = clusters_in_ground
            .into_iter()
            .filter(|cluster| cluster.samples > *context.minimum_samples_per_cluster)
            .collect();
        context.clusters_in_ground.fill_if_subscribed(|| {
            clusters_in_ground
                .iter()
                .map(|cluster| cluster.mean)
                .collect()
        });
        let positions = clusters_in_ground
            .into_iter()
            .map(|cluster| cluster.mean)
            .collect();
        Ok(MainOutputs {
            detected_feet: DetectedFeet { positions }.into(),
        })
    }
}

fn extract_segment_cluster_points(
    filtered_segments: &FilteredSegments,
    minimum_consecutive_segments: usize,
    minimum_luminance_standard_deviation: f32,
    balls: &[Ball],
    line_data: &LineData,
    camera_matrix: &CameraMatrix,
) -> Vec<ClusterPoint> {
    filtered_segments
        .scan_grid
        .vertical_scan_lines
        .iter()
        .filter_map(|scan_line| {
            let cluster = find_last_consecutive_cluster(
                scan_line,
                line_data,
                balls,
                minimum_consecutive_segments,
            )?;
            let luminances: Vec<_> = cluster
                .iter()
                .map(|segment| segment.color.y as f32)
                .collect();
            let luminance_standard_deviation = standard_deviation(&luminances, mean(&luminances));
            if luminance_standard_deviation < minimum_luminance_standard_deviation {
                return None;
            }
            let pixel_coordinates = point![scan_line.position, cluster.last().unwrap().end];
            let position_in_ground = camera_matrix
                .pixel_to_ground(pixel_coordinates.map(|x| x as f32))
                .ok()?;
            let point = ClusterPoint {
                pixel_coordinates,
                position_in_ground,
            };
            Some(point)
        })
        .collect()
}

fn find_last_consecutive_cluster(
    scan_line: &ScanLine,
    line_data: &LineData,
    balls: &[Ball],
    minimum_consecutive_segments: usize,
) -> Option<Vec<Segment>> {
    let filtered_segments = scan_line.segments.iter().filter(|segment| {
        let is_on_line = line_data
            .used_vertical_filtered_segments
            .contains(&point![scan_line.position, segment.start]);
        let is_on_ball = balls.iter().any(|ball| {
            ball.image_location
                .contains(point![scan_line.position as f32, segment.center() as f32])
        });
        !is_on_line && !is_on_ball
    });
    let consecutive_segments = filtered_segments
        .tuple_windows()
        .group_by(|(first, second)| first.end == second.start);
    consecutive_segments
        .into_iter()
        .filter_map(|(is_consecutive, group)| if is_consecutive { Some(group) } else { None })
        .filter_map(|mut group| {
            let first_window = group.next().unwrap();
            let mut consecutive_segments = vec![*first_window.0, *first_window.1];
            consecutive_segments.extend(group.map(|(_first, second)| *second));
            let last_edge_type = consecutive_segments.last().unwrap().end_edge_type;
            let last_segement_reaches_border =
                matches!(last_edge_type, EdgeType::ImageBorder | EdgeType::LimbBorder);
            if consecutive_segments.len() > minimum_consecutive_segments
                && !last_segement_reaches_border
            {
                Some(consecutive_segments)
            } else {
                None
            }
        })
        .last()
}

fn cluster_scored_cluster_points(
    cluster_points: Vec<ClusterPoint>,
    maximum_cluster_distance: f32,
) -> Vec<CountedCluster> {
    let mut clusters: Vec<CountedCluster> = Vec::new();
    for point in cluster_points {
        let nearest_cluster = clusters
            .iter_mut()
            .filter_map(|cluster| {
                let distance = distance(&cluster.mean, &point.position_in_ground);
                if distance < maximum_cluster_distance {
                    Some((cluster, distance))
                } else {
                    None
                }
            })
            .min_by(|(_, left_distance), (_, right_distance)| {
                left_distance.total_cmp(right_distance)
            });
        match nearest_cluster {
            Some((cluster, _)) => cluster.push(point.position_in_ground),
            None => clusters.push(CountedCluster {
                mean: point.position_in_ground,
                samples: 1,
            }),
        }
    }
    clusters
}
