use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Robot};
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use geometry::{convex_hull::reduce_to_convex_hull, is_inside_polygon::is_inside_polygon};
use linear_algebra::{point, Isometry3, Point2, Point3, Vector3};
use serde::{Deserialize, Serialize};
use types::{robot_kinematics::RobotKinematics, sensor_data::SensorData};

#[derive(Deserialize, Serialize)]
pub struct ZeroMomentPointProvider {
    linear_acceleration_filter: LowPassFilter<Vector3<Robot>>,
    number_of_frames_zero_moment_point_has_been_outside_support_polygon: i32,
    number_of_frames_center_of_mass_has_been_outside_support_polygon: i32,
}
#[context]
pub struct CreationContext {
    linear_acceleration_low_pass_factor:
        Parameter<f32, "zero_moment_point.linear_acceleration_low_pass_factor">,
}

#[context]
pub struct CycleContext {
    robot_to_ground: RequiredInput<Option<Isometry3<Robot, Ground>>, "robot_to_ground?">,

    center_of_mass: Input<Point3<Robot>, "center_of_mass">,
    robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    sensor_data: Input<SensorData, "sensor_data">,

    gravity_acceleration: Parameter<f32, "physical_constants.gravity_acceleration">,

    number_of_frames_center_of_mass_has_been_outside_support_polygon:
        AdditionalOutput<i32, "number_of_frames_center_of_mass_has_been_outside_support_polygon">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub zero_moment_point: MainOutput<Point2<Ground>>,
    pub number_of_frames_zero_moment_point_has_been_outside_support_polygon: MainOutput<i32>,
}

impl ZeroMomentPointProvider {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            linear_acceleration_filter: LowPassFilter::with_smoothing_factor(
                Vector3::zeros(),
                *context.linear_acceleration_low_pass_factor,
            ),
            number_of_frames_zero_moment_point_has_been_outside_support_polygon: 0,
            number_of_frames_center_of_mass_has_been_outside_support_polygon: 0,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        self.linear_acceleration_filter.update(
            context
                .sensor_data
                .inertial_measurement_unit
                .linear_acceleration,
        );

        let center_of_mass_in_ground = context.robot_to_ground * *context.center_of_mass;

        let linear_acceleration = context.robot_to_ground * self.linear_acceleration_filter.state();

        let zero_moment_point = center_of_mass_in_ground.xy()
            + linear_acceleration.xy() * center_of_mass_in_ground.z()
                / *context.gravity_acceleration;

        let left_sole_to_ground =
            *context.robot_to_ground * context.robot_kinematics.left_leg.sole_to_robot;
        let right_sole_to_ground =
            *context.robot_to_ground * context.robot_kinematics.right_leg.sole_to_robot;
        let left_outline = [
            point![-0.05457, -0.015151, 0.0],
            point![-0.050723, -0.021379, 0.0],
            point![-0.04262, -0.030603, 0.0],
            point![-0.037661, -0.033714, 0.0],
            point![-0.03297, -0.034351, 0.0],
            point![0.0577, -0.038771, 0.0],
            point![0.063951, -0.038362, 0.0],
            point![0.073955, -0.03729, 0.0],
            point![0.079702, -0.03532, 0.0],
            point![0.084646, -0.033221, 0.0],
            point![0.087648, -0.031482, 0.0],
            point![0.091805, -0.027692, 0.0],
            point![0.094009, -0.024299, 0.0],
            point![0.096868, -0.018802, 0.0],
            point![0.099419, -0.01015, 0.0],
            point![0.100097, -0.001573, 0.0],
            point![0.098991, 0.008695, 0.0],
            point![0.097014, 0.016504, 0.0],
            point![0.093996, 0.02418, 0.0],
            point![0.090463, 0.02951, 0.0],
            point![0.084545, 0.0361, 0.0],
            point![0.079895, 0.039545, 0.0],
            point![0.074154, 0.042654, 0.0],
            point![0.065678, 0.046145, 0.0],
            point![0.057207, 0.047683, 0.0],
            point![0.049911, 0.048183, 0.0],
            point![-0.031248, 0.051719, 0.0],
            point![-0.03593, 0.049621, 0.0],
            point![-0.040999, 0.045959, 0.0],
            point![-0.045156, 0.042039, 0.0],
            point![-0.04905, 0.037599, 0.0],
            point![-0.054657, 0.029814, 0.0],
        ];

        let sole_in_ground = left_outline
            .into_iter()
            .map(|point| (left_sole_to_ground * point).xy())
            .chain(left_outline.into_iter().map(|point| {
                (right_sole_to_ground * point![point.x(), -point.y(), point.z()]).xy()
            }))
            .collect::<Vec<_>>();
        let convex_hull = reduce_to_convex_hull(&sole_in_ground, false);

        if is_inside_polygon(&convex_hull, &zero_moment_point) {
            self.number_of_frames_zero_moment_point_has_been_outside_support_polygon = 0;
        } else {
            self.number_of_frames_zero_moment_point_has_been_outside_support_polygon += 1;
        }
        if is_inside_polygon(&convex_hull, &center_of_mass_in_ground.xy()) {
            self.number_of_frames_center_of_mass_has_been_outside_support_polygon = 0;
        } else {
            self.number_of_frames_center_of_mass_has_been_outside_support_polygon += 1;
        }

        context
            .number_of_frames_center_of_mass_has_been_outside_support_polygon
            .fill_if_subscribed(|| {
                self.number_of_frames_center_of_mass_has_been_outside_support_polygon
            });

        Ok(MainOutputs {
            zero_moment_point: zero_moment_point.into(),
            number_of_frames_zero_moment_point_has_been_outside_support_polygon: self
                .number_of_frames_zero_moment_point_has_been_outside_support_polygon
                .into(),
        })
    }
}
