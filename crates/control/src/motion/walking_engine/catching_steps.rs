use coordinate_systems::{Robot, Walk};
use linear_algebra::{point, vector, Isometry3, Orientation3, Point2, Point3, Vector3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::support_foot::Side;

use super::{feet::robot_to_walk, step_state::StepState, CycleContext};

struct Horizontal;

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum CatchingSteps {
    #[default]
    Inactive,
    Active {
        swing_foot_t0: Point3<Walk>,
        support_foot_t0: Point3<Walk>,
        adjustment: Vector3<Walk>,
    },
}

impl CatchingSteps {
    pub fn advance(self, context: &CycleContext, step: &StepState) -> Self {
        let parameters = &context.parameters.catching_steps;

        let left_sole_to_robot = context.robot_kinematics.left_sole_to_robot;
        let right_sole_to_robot = context.robot_kinematics.right_sole_to_robot;

        let left_toe = project_robot_to_ground(
            context,
            left_sole_to_robot * point![parameters.foot_support_forward, 0.0, 0.0],
        );
        let left_heel = project_robot_to_ground(
            context,
            left_sole_to_robot * point![parameters.foot_support_backward, 0.0, 0.0],
        );
        let right_toe = project_robot_to_ground(
            context,
            right_sole_to_robot * point![parameters.foot_support_forward, 0.0, 0.0],
        );
        let right_heel = project_robot_to_ground(
            context,
            right_sole_to_robot * point![parameters.foot_support_backward, 0.0, 0.0],
        );

        let forward_balance_limit = left_toe.x().max(right_toe.x());
        let backward_balance_limit = left_heel.x().min(right_heel.x());

        let center_of_mass = project_robot_to_ground(context, *context.center_of_mass);

        let center_of_mass_is_outside_balance_limits =
            !(backward_balance_limit..forward_balance_limit).contains(&center_of_mass.x());

        let swing_foot_target = robot_to_walk(context)
            * projected_ground_to_robot(context, step.support_side, center_of_mass);

        let last_adjustment = match self {
            Self::Active { adjustment, .. } => adjustment,
            _ => Vector3::zeros(),
        };

        let target = vector![
            swing_foot_target.x().clamp(
                -parameters.max_adjustment.x(),
                parameters.max_adjustment.x()
            ),
            swing_foot_target.y().clamp(
                -parameters.max_adjustment.y(),
                parameters.max_adjustment.y()
            ),
            swing_foot_target.z().clamp(
                -parameters.max_adjustment.z(),
                parameters.max_adjustment.z()
            ),
        ];
        let direction = target - last_adjustment;
        let limited_adjustment = if direction != Vector3::zeros() {
            last_adjustment
                + direction.normalize()
                    * direction.norm().clamp(
                        -parameters.max_adjustment_delta,
                        parameters.max_adjustment_delta,
                    )
        } else {
            last_adjustment
        };

        match self {
            CatchingSteps::Inactive => {
                if center_of_mass_is_outside_balance_limits {
                    let feet = step.feet_at(context.cycle_time.start_time, context.parameters);
                    CatchingSteps::Active {
                        swing_foot_t0: feet.swing_foot,
                        support_foot_t0: feet.support_foot,
                        adjustment: limited_adjustment,
                    }
                } else {
                    CatchingSteps::Inactive
                }
            }
            CatchingSteps::Active {
                swing_foot_t0,
                support_foot_t0,
                ..
            } => {
                if center_of_mass_is_outside_balance_limits {
                    CatchingSteps::Active {
                        swing_foot_t0,
                        support_foot_t0,
                        adjustment: limited_adjustment,
                    }
                } else {
                    self
                }
            }
        }
    }
}

fn project_robot_to_ground(context: &CycleContext, point: Point3<Robot>) -> Point2<Horizontal> {
    let imu_roll_pitch = context.sensor_data.inertial_measurement_unit.roll_pitch;
    let imu_orientation =
        Orientation3::from_euler_angles(imu_roll_pitch.x, imu_roll_pitch.y, 0.0).inverse();
    let robot_to_horizontal =
        Isometry3::<Robot, Horizontal>::from_parts(Vector3::zeros(), imu_orientation);
    let point_in_parallel = robot_to_horizontal * point;
    point_in_parallel.xy()
}

fn projected_ground_to_robot(
    context: &CycleContext,
    support_side: Side,
    point: Point2<Horizontal>,
) -> Point3<Robot> {
    let imu_roll_pitch = context.sensor_data.inertial_measurement_unit.roll_pitch;
    let imu_orientation =
        Orientation3::from_euler_angles(imu_roll_pitch.x, imu_roll_pitch.y, 0.0).inverse();
    let robot_to_horizontal =
        Isometry3::<Robot, Horizontal>::from_parts(Vector3::zeros(), imu_orientation);

    let support_sole = match support_side {
        Side::Left => context.robot_kinematics.left_sole_to_robot.as_pose(),
        Side::Right => context.robot_kinematics.right_sole_to_robot.as_pose(),
    };

    let support_sole_in_parallel = robot_to_horizontal * support_sole;
    robot_to_horizontal.inverse()
        * point![
            point.x(),
            point.y(),
            support_sole_in_parallel.position().z()
        ]
}
