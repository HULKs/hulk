use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, Parameter, RequiredInput};

pub struct SonarFilter {}

#[context]
pub struct NewContext {
    pub low_pass_filter_coefficient:
        Parameter<f32, "control/sonar_filter/low_pass_filter_coefficient">,
    pub maximal_detectable_distance:
        Parameter<f32, "control/sonar_filter/maximal_detectable_distance">,
    pub maximal_reliable_distance: Parameter<f32, "control/sonar_filter/maximal_reliable_distance">,
    pub minimal_reliable_distance: Parameter<f32, "control/sonar_filter/minimal_reliable_distance">,
    pub sensor_angle: Parameter<f32, "control/sonar_obstacle/sensor_angle">,
}

#[context]
pub struct CycleContext {
    pub sonar_values: AdditionalOutput<SonarValues, "sonar_values">,

    pub low_pass_filter_coefficient:
        Parameter<f32, "control/sonar_filter/low_pass_filter_coefficient">,
    pub maximal_detectable_distance:
        Parameter<f32, "control/sonar_filter/maximal_detectable_distance">,
    pub maximal_reliable_distance: Parameter<f32, "control/sonar_filter/maximal_reliable_distance">,
    pub minimal_reliable_distance: Parameter<f32, "control/sonar_filter/minimal_reliable_distance">,
    pub sensor_angle: Parameter<f32, "control/sonar_obstacle/sensor_angle">,

    pub fall_state: RequiredInput<FallState, "fall_state">,
    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sonar_obstacle: MainOutput<SonarObstacle>,
}

impl SonarFilter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
