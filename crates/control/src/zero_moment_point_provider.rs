use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Robot};
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use linear_algebra::{point, Isometry3, Point3, Vector3};
use serde::{Deserialize, Serialize};
use types::sensor_data::SensorData;

#[derive(Deserialize, Serialize)]
pub struct ZeroMomentPointProvider {
    linear_acceleration_filter: LowPassFilter<Vector3<Robot>>,
}

#[context]
pub struct CreationContext {
    linear_acceleration_low_pass_factor:
        Parameter<f32, "zero_moment_point.linear_acceleration_low_pass_factor">,
}

#[context]
pub struct CycleContext {
    center_of_mass: Input<Point3<Robot>, "center_of_mass">,
    sensor_data: Input<SensorData, "sensor_data">,

    gravity_acceleration: Parameter<f32, "physical_constants.gravity_acceleration">,

    robot_to_ground: RequiredInput<Option<Isometry3<Robot, Ground>>, "robot_to_ground?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub zero_moment_point: MainOutput<Point3<Ground>>,
}

impl ZeroMomentPointProvider {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            linear_acceleration_filter: LowPassFilter::with_smoothing_factor(
                Vector3::zeros(),
                *context.linear_acceleration_low_pass_factor,
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        self.linear_acceleration_filter.update(
            context
                .sensor_data
                .inertial_measurement_unit
                .linear_acceleration,
        );

        let center_of_mass_in_ground = context.robot_to_ground * *context.center_of_mass;
        let x_center_of_mass = center_of_mass_in_ground.x();
        let y_center_of_mass = center_of_mass_in_ground.y();
        let z_center_of_mass = center_of_mass_in_ground.z();

        let linear_acceleration = context.robot_to_ground * self.linear_acceleration_filter.state();
        let x_acceleration_parallel_to_ground = linear_acceleration.x();
        let y_acceleration_parallel_to_ground = linear_acceleration.y();
        let x_zero_moment_point_in_robot = ((x_center_of_mass * context.gravity_acceleration)
            + (x_acceleration_parallel_to_ground * z_center_of_mass))
            / context.gravity_acceleration;
        let y_zero_moment_point_in_robot = ((y_center_of_mass * context.gravity_acceleration)
            + (y_acceleration_parallel_to_ground * z_center_of_mass))
            / context.gravity_acceleration;

        let zero_moment_point: Point3<Ground> = point![
            x_zero_moment_point_in_robot,
            y_zero_moment_point_in_robot,
            0.0
        ];
        Ok(MainOutputs {
            zero_moment_point: zero_moment_point.into(),
        })
    }
}
