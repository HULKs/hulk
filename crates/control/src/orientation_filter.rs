use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::UnitComplex;
use types::{CycleTime, OrientationFilterParameters, SensorData, SolePressure, SupportFoot};

pub struct OrientationFilter {
    orientation_filter: filtering::orientation_filter::OrientationFilter,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub sole_pressure: Input<SolePressure, "sole_pressure">,
    pub support_foot: Input<SupportFoot, "support_foot">,

    pub orientation_filter_configuration:
        Parameter<OrientationFilterParameters, "orientation_filter">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_orientation: MainOutput<UnitComplex<f32>>,
}

impl OrientationFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            orientation_filter: Default::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let measured_acceleration = context
            .sensor_data
            .inertial_measurement_unit
            .linear_acceleration;
        let measured_angular_velocity = context
            .sensor_data
            .inertial_measurement_unit
            .angular_velocity;
        let cycle_duration = context.cycle_time.last_cycle_duration;
        self.orientation_filter.update(
            measured_acceleration,
            measured_angular_velocity,
            context.sole_pressure.left,
            context.sole_pressure.right,
            cycle_duration.as_secs_f32(),
            context.orientation_filter_configuration,
        );

        Ok(MainOutputs {
            robot_orientation: self.orientation_filter.yaw().into(),
        })
    }
}
