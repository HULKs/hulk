use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Robot};
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use linear_algebra::{Isometry3, Point2, Point3, Vector3};
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
    pub zero_moment_point: MainOutput<Point2<Ground>>,
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

        let linear_acceleration = context.robot_to_ground * self.linear_acceleration_filter.state();

        let zero_moment_point = center_of_mass_in_ground.xy()
            + linear_acceleration.xy() * center_of_mass_in_ground.z()
                / *context.gravity_acceleration;
        Ok(MainOutputs {
            zero_moment_point: zero_moment_point.into(),
        })
    }
}
