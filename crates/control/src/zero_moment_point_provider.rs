use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Robot};
use framework::MainOutput;
use linear_algebra::{point, Isometry3, Point3, Vector3};
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
    robot_to_ground: RequiredInput<Option<Isometry3<Robot, Ground>>, "robot_to_ground?">,
    filtered_linear_acceleration: Input<Vector3<Robot>, "filtered_linear_acceleration">,
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

        let center_of_mass_in_ground = context.robot_to_ground * *context.center_of_mass;
        let x_com = center_of_mass_in_ground.x();
        let y_com = center_of_mass_in_ground.y();
        let z_com = center_of_mass_in_ground.z();

        let _unfitlered_imu_rotated_parallel_to_ground = context.robot_to_ground
            * context
                .sensor_data
                .inertial_measurement_unit
                .linear_acceleration;
        let imu_rotated_parallel_to_ground =
            context.robot_to_ground * *context.filtered_linear_acceleration;
        let x_acceleration_parallel_to_ground = imu_rotated_parallel_to_ground.x();
        let y_acceleration_parallel_to_ground = imu_rotated_parallel_to_ground.y();
        let x_zero_moment_point_in_robot = ((x_com * GRAVITATIONAL_CONSTANT)
            + (x_acceleration_parallel_to_ground * z_com))
            / GRAVITATIONAL_CONSTANT;
        let y_zero_moment_point_in_robot = ((y_com * GRAVITATIONAL_CONSTANT)
            + (y_acceleration_parallel_to_ground * z_com))
            / GRAVITATIONAL_CONSTANT;

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
