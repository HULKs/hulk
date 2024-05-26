use color_eyre::Result;

use geometry::convex_hull::reduce_to_convex_hull;

use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{LeftElbow, LeftSole, LeftThigh, LeftWrist, Robot};
use framework::MainOutput;
use linear_algebra::{point, Isometry3, Point3};
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
            context.robot_kinematics.left_arm.wrist_to_robot,
            context.camera_matrix,
            context.wrist_bounding_polygon,
            true,
        );
        let right_lower_arm_limb = project_bounding_polygon(
            context.robot_kinematics.right_arm.wrist_to_robot,
            context.camera_matrix,
            &mirror_polygon(context.wrist_bounding_polygon),
            true,
        );
        let left_upper_arm_limb = project_bounding_polygon(
            context.robot_kinematics.left_arm.elbow_to_robot,
            context.camera_matrix,
            context.upper_arm_bounding_polygon,
            true,
        );
        let right_upper_arm_limb = project_bounding_polygon(
            context.robot_kinematics.right_arm.elbow_to_robot,
            context.camera_matrix,
            &mirror_polygon(context.upper_arm_bounding_polygon),
            true,
        );
        let left_knee_limb = project_bounding_polygon(
            context.robot_kinematics.left_leg.thigh_to_robot,
            context.camera_matrix,
            context.thigh_bounding_polygon,
            true,
        );
        let right_knee_limb = project_bounding_polygon(
            context.robot_kinematics.right_leg.thigh_to_robot,
            context.camera_matrix,
            &mirror_polygon(context.thigh_bounding_polygon),
            true,
        );
        let left_foot_limb = project_bounding_polygon(
            context.robot_kinematics.left_leg.sole_to_robot,
            context.camera_matrix,
            context.sole_bounding_polygon,
            true,
        );
        let right_foot_limb = project_bounding_polygon(
            context.robot_kinematics.right_leg.sole_to_robot,
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
            reduce_to_convex_hull(&points, true)
        } else {
            points
        },
    }
}

fn mirror_polygon<From, To>(polygon: &[Point3<From>]) -> Vec<Point3<To>> {
    polygon
        .iter()
        .map(|point| point![point.x(), -point.y(), point.z()])
        .collect()
}
