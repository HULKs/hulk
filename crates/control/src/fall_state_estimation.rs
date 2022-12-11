use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use nalgebra::{Vector2, Vector3};
use types::{
    configuration::FallStateEstimation as FallStateEstimationConfiguration, FallState, SensorData,
};

pub struct FallStateEstimation {}

#[context]
pub struct CreationContext {
    pub fall_state_estimation:
        Parameter<FallStateEstimationConfiguration, "control/fall_state_estimation">,
}

#[context]
pub struct CycleContext {
    pub backward_gravitational_difference:
        AdditionalOutput<f32, "backward_gravitational_difference">,
    pub filtered_angular_velocity: AdditionalOutput<Vector3<f32>, "filtered_angular_velocity">,
    pub filtered_linear_acceleration:
        AdditionalOutput<Vector3<f32>, "filtered_linear_acceleration">,
    pub filtered_roll_pitch: AdditionalOutput<Vector2<f32>, "filtered_roll_pitch">,
    pub forward_gravitational_difference: AdditionalOutput<f32, "forward_gravitational_difference">,

    pub fall_state_estimation:
        Parameter<FallStateEstimationConfiguration, "control/fall_state_estimation">,

    pub has_ground_contact: Input<bool, "has_ground_contact">,
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_state: MainOutput<Option<FallState>>,
}

impl FallStateEstimation {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
