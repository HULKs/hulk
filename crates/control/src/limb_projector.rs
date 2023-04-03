use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{Isometry3, Matrix2, Point2, Point3};
use projection::Projection;
use types::{CameraMatrices, CameraMatrix, Limb, ProjectedLimbs, RobotKinematics};

pub struct LimbProjector {}

#[context]
pub struct CreationContext {
    pub foot_bounding_polygon: Parameter<Vec<Point3<f32>>, "projected_limbs.foot_bounding_polygon">,
    pub knee_bounding_polygon: Parameter<Vec<Point3<f32>>, "projected_limbs.knee_bounding_polygon">,
    pub lower_arm_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "projected_limbs.lower_arm_bounding_polygon">,
    pub torso_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "projected_limbs.torso_bounding_polygon">,
    pub upper_arm_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "projected_limbs.upper_arm_bounding_polygon">,
}

#[context]
pub struct CycleContext {
    pub camera_matrices: RequiredInput<Option<CameraMatrices>, "camera_matrices?">,
    pub robot_kinematics: Input<RobotKinematics, "robot_kinematics">,

    pub foot_bounding_polygon: Parameter<Vec<Point3<f32>>, "projected_limbs.foot_bounding_polygon">,
    pub knee_bounding_polygon: Parameter<Vec<Point3<f32>>, "projected_limbs.knee_bounding_polygon">,
    pub lower_arm_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "projected_limbs.lower_arm_bounding_polygon">,
    pub torso_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "projected_limbs.torso_bounding_polygon">,
    pub upper_arm_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "projected_limbs.upper_arm_bounding_polygon">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub projected_limbs: MainOutput<Option<ProjectedLimbs>>,
}

impl LimbProjector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let camera_matrix = &context.camera_matrices.bottom;
        let torso_limb = project_bounding_polygon(
            Isometry3::identity(),
            camera_matrix,
            context.torso_bounding_polygon,
            false,
        );
        let left_lower_arm_limb = project_bounding_polygon(
            context.robot_kinematics.left_wrist_to_robot,
            camera_matrix,
            context.lower_arm_bounding_polygon,
            true,
        );
        let right_lower_arm_limb = project_bounding_polygon(
            context.robot_kinematics.right_wrist_to_robot,
            camera_matrix,
            context.lower_arm_bounding_polygon,
            true,
        );
        let left_upper_arm_limb = project_bounding_polygon(
            context.robot_kinematics.left_elbow_to_robot,
            camera_matrix,
            context.upper_arm_bounding_polygon,
            true,
        );
        let right_upper_arm_limb = project_bounding_polygon(
            context.robot_kinematics.right_elbow_to_robot,
            camera_matrix,
            context.upper_arm_bounding_polygon,
            true,
        );
        let left_knee_limb = project_bounding_polygon(
            context.robot_kinematics.left_thigh_to_robot,
            camera_matrix,
            context.knee_bounding_polygon,
            true,
        );
        let right_knee_limb = project_bounding_polygon(
            context.robot_kinematics.right_thigh_to_robot,
            camera_matrix,
            context.knee_bounding_polygon,
            true,
        );
        let left_foot_limb = project_bounding_polygon(
            context.robot_kinematics.left_sole_to_robot,
            camera_matrix,
            context.foot_bounding_polygon,
            true,
        );
        let right_foot_limb = project_bounding_polygon(
            context.robot_kinematics.right_sole_to_robot,
            camera_matrix,
            context.foot_bounding_polygon,
            true,
        );

        Ok(MainOutputs {
            projected_limbs: Some(ProjectedLimbs {
                top: vec![],
                bottom: vec![
                    torso_limb,
                    left_lower_arm_limb,
                    right_lower_arm_limb,
                    left_upper_arm_limb,
                    right_upper_arm_limb,
                    left_knee_limb,
                    right_knee_limb,
                    left_foot_limb,
                    right_foot_limb,
                ],
            })
            .into(),
        })
    }
}
fn project_bounding_polygon(
    limb_to_robot: Isometry3<f32>,
    camera_matrix: &CameraMatrix,
    bounding_polygon: &[Point3<f32>],
    use_convex_hull: bool,
) -> Limb {
    let points: Vec<_> = bounding_polygon
        .iter()
        .filter_map(|point| camera_matrix.robot_to_pixel(limb_to_robot * point).ok())
        .collect();
    Limb {
        pixel_polygon: if use_convex_hull {
            reduce_to_convex_hull(&points)
        } else {
            points
        },
    }
}

fn reduce_to_convex_hull(points: &[Point2<f32>]) -> Vec<Point2<f32>> {
    // https://en.wikipedia.org/wiki/Gift_wrapping_algorithm
    // Modification: This implementation iterates from left to right until a smaller x value is found
    if points.is_empty() {
        return vec![];
    }
    let mut point_on_hull = *points.iter().min_by(|a, b| a.x.total_cmp(&b.x)).unwrap();
    let mut convex_hull = vec![];
    loop {
        convex_hull.push(point_on_hull);
        let mut candidate_end_point = points[0];
        for point in points.iter() {
            let last_point_on_hull_to_candidate_end_point = candidate_end_point - point_on_hull;
            let last_point_on_hull_to_point = point - point_on_hull;
            let determinant = Matrix2::from_columns(&[
                last_point_on_hull_to_candidate_end_point,
                last_point_on_hull_to_point,
            ])
            .determinant();
            let point_is_left_of_candidate_end_point = determinant < 0.0;
            if candidate_end_point == point_on_hull || point_is_left_of_candidate_end_point {
                candidate_end_point = *point;
            }
        }
        // begin of modification
        let has_smaller_x = candidate_end_point.x < point_on_hull.x;
        if has_smaller_x {
            break;
        }
        // end of modification
        point_on_hull = candidate_end_point;
        if candidate_end_point == *convex_hull.first().unwrap() {
            break;
        }
    }
    convex_hull
}
