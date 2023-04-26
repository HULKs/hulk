use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    ArmJoints, BodyJoints, BodyJointsCommand, CycleTime, Joints, LegJoints, MotionSelection,
    SensorData,
};

pub struct EnergySavingStand {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub arm_stiffness: Parameter<f32, "energy_saving_stand.arm_stiffness">,
    pub leg_stiffness: Parameter<f32, "energy_saving_stand.leg_stiffness">,
    pub energy_saving_stand_pose: Parameter<Joints<f32>, "energy_saving_stand.pose">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub energy_saving_stand_command: MainOutput<BodyJointsCommand<f32>>,
}

impl EnergySavingStand {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs {
            energy_saving_stand_command: BodyJointsCommand {
                positions: BodyJoints::from(*context.energy_saving_stand_pose),
                stiffnesses: BodyJoints {
                    left_arm: ArmJoints::fill(*context.arm_stiffness),
                    right_arm: ArmJoints::fill(*context.arm_stiffness),
                    left_leg: LegJoints::fill(*context.leg_stiffness),
                    right_leg: LegJoints::fill(*context.leg_stiffness),
                },
            }
            .into(),
        })
    }
}
