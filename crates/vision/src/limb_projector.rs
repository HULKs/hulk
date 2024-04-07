use color_eyre::Result;

use nalgebra::Matrix2;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{LeftElbow, LeftSole, LeftThigh, LeftWrist, Robot};
use framework::MainOutput;
use linear_algebra::{point, Isometry3, Point2, Point3};
use projection::{camera_matrix::CameraMatrix, Projection};
use types::{
    limb::{Limb, ProjectedLimbs},
    robot_kinematics::RobotKinematics,
};

#[derive(Deserialize, Serialize)]
pub struct LimbProjector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    robot_kinematics: Input<RobotKinematics, "Control", "robot_kinematics">,

    enable: Parameter<bool, "projected_limbs.$cycler_instance.enable">,
    sole_bounding_polygon:
        Parameter<Vec<Point3<LeftSole>>, "projected_limbs.foot_bounding_polygon">,
    thigh_bounding_polygon:
        Parameter<Vec<Point3<LeftThigh>>, "projected_limbs.knee_bounding_polygon">,
    wrist_bounding_polygon:
        Parameter<Vec<Point3<LeftWrist>>, "projected_limbs.lower_arm_bounding_polygon">,
    torso_bounding_polygon: Parameter<Vec<Point3<Robot>>, "projected_limbs.torso_bounding_polygon">,
    upper_arm_bounding_polygon:
        Parameter<Vec<Point3<LeftElbow>>, "projected_limbs.upper_arm_bounding_polygon">,
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
        if !context.enable {
            return Ok(MainOutputs {
                projected_limbs: Default::default(),
            });
        }
        let torso_limb = project_bounding_polygon(
            Isometry3::identity(),
            context.camera_matrix,
            context.torso_bounding_polygon,
            false,
        );
        let left_lower_arm_limb = project_bounding_polygon(
            context.robot_kinematics.left_wrist_to_robot,
            context.camera_matrix,
            context.wrist_bounding_polygon,
            true,
        );
        let right_lower_arm_limb = project_bounding_polygon(
            context.robot_kinematics.right_wrist_to_robot,
            context.camera_matrix,
            &mirror_polygon(context.wrist_bounding_polygon),
            true,
        );
        let left_upper_arm_limb = project_bounding_polygon(
            context.robot_kinematics.left_elbow_to_robot,
            context.camera_matrix,
            context.upper_arm_bounding_polygon,
            true,
        );
        let right_upper_arm_limb = project_bounding_polygon(
            context.robot_kinematics.right_elbow_to_robot,
            context.camera_matrix,
            &mirror_polygon(context.upper_arm_bounding_polygon),
            true,
        );
        let left_knee_limb = project_bounding_polygon(
            context.robot_kinematics.left_thigh_to_robot,
            context.camera_matrix,
            context.thigh_bounding_polygon,
            true,
        );
        let right_knee_limb = project_bounding_polygon(
            context.robot_kinematics.right_thigh_to_robot,
            context.camera_matrix,
            &mirror_polygon(context.thigh_bounding_polygon),
            true,
        );
        let left_foot_limb = project_bounding_polygon(
            context.robot_kinematics.left_sole_to_robot,
            context.camera_matrix,
            context.sole_bounding_polygon,
            true,
        );
        let right_foot_limb = project_bounding_polygon(
            context.robot_kinematics.right_sole_to_robot,
            context.camera_matrix,
            &mirror_polygon(context.sole_bounding_polygon),
            true,
        );

        let limbs = vec![
            torso_limb,
            left_lower_arm_limb,
            right_lower_arm_limb,
            left_upper_arm_limb,
            right_upper_arm_limb,
            left_knee_limb,
            right_knee_limb,
            left_foot_limb,
            right_foot_limb,
        ];
        Ok(MainOutputs {
            projected_limbs: Some(ProjectedLimbs { limbs }).into(),
        })
    }
}

fn project_bounding_polygon<Frame>(
    limb_to_robot: Isometry3<Frame, Robot>,
    camera_matrix: &CameraMatrix,
    bounding_polygon: &[Point3<Frame>],
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

fn reduce_to_convex_hull<Frame>(points: &[Point2<Frame>]) -> Vec<Point2<Frame>>
where
    Frame: Copy,
{
    // https://en.wikipedia.org/wiki/Gift_wrapping_algorithm
    // Modification: This implementation iterates from left to right until a smaller x value is found
    if points.is_empty() {
        return vec![];
    }
    let mut point_on_hull = *points
        .iter()
        .min_by(|a, b| a.x().total_cmp(&b.x()))
        .unwrap();
    let mut convex_hull = vec![];
    loop {
        convex_hull.push(point_on_hull);
        let mut candidate_end_point = points[0];
        for point in points.iter() {
            let last_point_on_hull_to_candidate_end_point = candidate_end_point - point_on_hull;
            let last_point_on_hull_to_point = *point - point_on_hull;
            let determinant = Matrix2::from_columns(&[
                last_point_on_hull_to_candidate_end_point.inner,
                last_point_on_hull_to_point.inner,
            ])
            .determinant();
            let point_is_left_of_candidate_end_point = determinant < 0.0;
            if candidate_end_point == point_on_hull || point_is_left_of_candidate_end_point {
                candidate_end_point = *point;
            }
        }
        // begin of modification
        let has_smaller_x = candidate_end_point.x() < point_on_hull.x();
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

fn mirror_polygon<From, To>(polygon: &[Point3<From>]) -> Vec<Point3<To>> {
    polygon
        .iter()
        .map(|point| point![point.x(), -point.y(), point.z()])
        .collect()
}
