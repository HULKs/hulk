use context_attribute::context;
use framework::{MainOutput, OptionalInput, Parameter, PersistentState};

pub struct StepPlanner {}

#[context]
pub struct NewContext {
    pub injected_step: Parameter<Option<Step>, "control/step_planner/injected_step">,
    pub inside_turn_ratio: Parameter<f32, "control/step_planner/inside_turn_ratio">,
    pub max_step_size: Parameter<Step, "control/step_planner/max_step_size">,
    pub max_step_size_backwards: Parameter<f32, "control/step_planner/max_step_size_backwards">,
    pub rotation_exponent: Parameter<f32, "control/step_planner/rotation_exponent">,
    pub translation_exponent: Parameter<f32, "control/step_planner/translation_exponent">,

    pub walk_return_offset: PersistentState<Step, "walk_return_offset">,
}

#[context]
pub struct CycleContext {
    pub motion_command: OptionalInput<MotionCommand, "motion_command?">,
    pub sensor_data: OptionalInput<SensorData, "sensor_data?">,
    pub support_foot: OptionalInput<SupportFoot, "support_foot?">,

    pub injected_step: Parameter<Option<Step>, "control/step_planner/injected_step">,
    pub inside_turn_ratio: Parameter<f32, "control/step_planner/inside_turn_ratio">,
    pub max_step_size: Parameter<Step, "control/step_planner/max_step_size">,
    pub max_step_size_backwards: Parameter<f32, "control/step_planner/max_step_size_backwards">,
    pub rotation_exponent: Parameter<f32, "control/step_planner/rotation_exponent">,
    pub translation_exponent: Parameter<f32, "control/step_planner/translation_exponent">,

    pub walk_return_offset: PersistentState<Step, "walk_return_offset">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub step_plan: MainOutput<Step>,
}

impl StepPlanner {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
