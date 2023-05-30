use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::Joints;

pub struct JointCalibrationProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub joint_calibration_offsets_deg: Parameter<Joints<f32>, "joint_calibration_offsets_deg">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub joint_calibration_offsets: MainOutput<Joints<f32>>,
}

impl JointCalibrationProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let joint_calibration_offsets =
            *context.joint_calibration_offsets_deg * std::f32::consts::PI / 180.0;

        Ok(MainOutputs {
            joint_calibration_offsets: joint_calibration_offsets.into(),
        })
    }
}
