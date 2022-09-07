use context_attribute::context;
use filtering::LowPassFilter;
use nalgebra::{Point3, Vector2, Vector3};

use framework::{AdditionalOutput, MainOutput, Parameter, RequiredInput};
use types::{FallState, RobotKinematics, SensorData};

// TODO: Seems wrong
pub struct FallStateEstimation {
    roll_pitch_filter: LowPassFilter<Vector2<f32>>,
    angular_velocity_filter: LowPassFilter<Vector3<f32>>,
    linear_acceleration_filter: LowPassFilter<Vector3<f32>>,
}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub fall_state_estimation: Parameter<FallStateEstimation, "control/fall_state_estimation">, // TODO: FallStateEstimation aus framework::configuration

    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
    pub has_ground_contact: RequiredInput<bool, "has_ground_contact">,

    pub filtered_linear_acceleration:
        AdditionalOutput<Vector3<f32>, "filtered_linear_acceleration">,
    pub filtered_angular_velocity: AdditionalOutput<Vector3<f32>, "filtered_angular_velocity">,
    pub filtered_roll_pitch: AdditionalOutput<Vector3<f32>, "filtered_roll_pitch">,
    pub forward_gravitational_difference:
        AdditionalOutput<Vector3<f32>, "forward_gravitational_difference">,
    pub backward_gravitational_difference:
        AdditionalOutput<Vector3<f32>, "backward_gravitational_difference">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_state: MainOutput<FallState>,
}

impl FallStateEstimation {
    /* TODO:
    fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            roll_pitch_filter: LowPassFilter::with_alpha(
                Vector2::zeros(),
                context.fall_state_estimation.roll_pitch_low_pass_factor,
            ),
            angular_velocity_filter: LowPassFilter::with_alpha(
                Vector3::zeros(),
                context
                    .fall_state_estimation
                    .angular_velocity_low_pass_factor,
            ),
            linear_acceleration_filter: LowPassFilter::with_alpha(
                Vector3::zeros(),
                context
                    .fall_state_estimation
                    .linear_acceleration_low_pass_factor,
            ),
        })
    }
    */
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
