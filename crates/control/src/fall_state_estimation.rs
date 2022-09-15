use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, Parameter, RequiredInput};

pub struct FallStateEstimation {}

#[context]
pub struct NewContext {
    pub fall_state_estimation: Parameter<
        crate::framework::configuration::FallStateEstimation,
        "control/fall_state_estimation",
    >,
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

    pub fall_state_estimation: Parameter<
        crate::framework::configuration::FallStateEstimation,
        "control/fall_state_estimation",
    >,

    pub has_ground_contact: RequiredInput<bool, "has_ground_contact">,
    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_state: MainOutput<FallState>,
}

impl FallStateEstimation {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
