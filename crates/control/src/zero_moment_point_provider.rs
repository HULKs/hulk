use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Robot};
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use geometry::{
    convex_hull::{reduce_to_convex_hull, Range},
    is_inside_polygon::is_inside_convex_hull,
};
use linear_algebra::{Isometry3, Point2, Point3, Vector3};
use serde::{Deserialize, Serialize};
use types::{
    robot_dimensions::{transform_left_sole_outline, transform_right_sole_outline},
    robot_kinematics::RobotKinematics,
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct ZeroMomentPointProvider {
    linear_acceleration_filter: LowPassFilter<Vector3<Robot>>,
    consecutive_cycles_zero_moment_point_outside_support_polygon: i32,
    consecutive_cycles_center_of_mass_outside_support_polygon: i32,
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

    consecutive_cycles_center_of_mass_outside_support_polygon:
        AdditionalOutput<i32, "consecutive_cycles_center_of_mass_outside_support_polygon">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub zero_moment_point: MainOutput<Point2<Ground>>,
    pub consecutive_cycles_zero_moment_point_outside_support_polygon: MainOutput<i32>,
}

impl ZeroMomentPointProvider {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            linear_acceleration_filter: LowPassFilter::with_smoothing_factor(
                Vector3::zeros(),
                *context.linear_acceleration_low_pass_factor,
            ),
            consecutive_cycles_zero_moment_point_outside_support_polygon: 0,
            consecutive_cycles_center_of_mass_outside_support_polygon: 0,
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

        let soles_in_ground = transform_left_sole_outline(left_sole_to_ground)
            .chain(transform_right_sole_outline(right_sole_to_ground))
            .map(|point| point.xy())
            .collect::<Vec<_>>();

        let soles_in_ground_hull = reduce_to_convex_hull(&soles_in_ground, Range::Full);

        if is_inside_convex_hull(&soles_in_ground_hull, &zero_moment_point) {
            self.consecutive_cycles_zero_moment_point_outside_support_polygon = 0;
        } else {
            self.consecutive_cycles_zero_moment_point_outside_support_polygon += 1;
        }
        if is_inside_convex_hull(&soles_in_ground_hull, &center_of_mass_in_ground.xy()) {
            self.consecutive_cycles_center_of_mass_outside_support_polygon = 0;
        } else {
            self.consecutive_cycles_center_of_mass_outside_support_polygon += 1;
        }

        context
            .consecutive_cycles_center_of_mass_outside_support_polygon
            .fill_if_subscribed(|| self.consecutive_cycles_center_of_mass_outside_support_polygon);

        Ok(MainOutputs {
            zero_moment_point: zero_moment_point.into(),
            consecutive_cycles_zero_moment_point_outside_support_polygon: self
                .consecutive_cycles_zero_moment_point_outside_support_polygon
                .into(),
        })
    }
}
