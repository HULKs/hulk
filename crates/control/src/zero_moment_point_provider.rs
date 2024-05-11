use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Robot};
use framework::MainOutput;
use linear_algebra::{point, Point3, Rotation3};
use serde::{Deserialize, Serialize};
use types::sensor_data::SensorData;

#[derive(Deserialize, Serialize)]
pub struct ZeroMomentPointProvider {}
#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    center_of_mass: Input<Point3<Robot>, "center_of_mass">,
    sensor_data: Input<SensorData, "sensor_data">,
}
#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub zero_moment_point: MainOutput<Point3<Ground>>,
}

impl ZeroMomentPointProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        const GRAVITATIONAL_CONSTANT: f32 = 9.81;

        let imu_orientation = Rotation3::<Ground, Robot>::from_euler_angles(
            context.sensor_data.inertial_measurement_unit.roll_pitch.x(),
            context.sensor_data.inertial_measurement_unit.roll_pitch.y(),
            0.0,
        );

        let y_com = context.center_of_mass.y();
        let x_com = context.center_of_mass.x();
        let z = context.center_of_mass.z();

        let imu_rotated_parallel_to_ground = imu_orientation.inverse()
            * context
                .sensor_data
                .inertial_measurement_unit
                .linear_acceleration;
        let x_hat = imu_rotated_parallel_to_ground.x();
        let y_hat = imu_rotated_parallel_to_ground.y();
        let x_zero_moment_point_in_robot =
            ((x_com * GRAVITATIONAL_CONSTANT) + (x_hat * z)) / GRAVITATIONAL_CONSTANT;
        let y_zero_moment_point_in_robot =
            ((y_com * GRAVITATIONAL_CONSTANT) + (y_hat * z)) / GRAVITATIONAL_CONSTANT;

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
