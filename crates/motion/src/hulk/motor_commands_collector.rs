use color_eyre::{Result, eyre::Ok};
use context_attribute::context;
use framework::MainOutput;
use kinematics::joints::{Joints, head::HeadJoints};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MotorCommandCollector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    head_target_joints_positions: Input<HeadJoints<f32>, "head_joints_command">,
    walking_target_joint_positions: Input<Option<Joints>, "walking_target_joint_positions?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub collected_target_joint_positions: MainOutput<Option<Joints<f32>>>,
}

impl MotorCommandCollector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }
    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let Some(walking_target_joint_positions) = context.walking_target_joint_positions else {
            return Ok(MainOutputs {
                collected_target_joint_positions: None.into(),
            });
        };

        let collected_target_joint_positions = Joints::from_head_and_body(
            *context.head_target_joints_positions,
            walking_target_joint_positions.body(),
        );

        Ok(MainOutputs {
            collected_target_joint_positions: Some(collected_target_joint_positions).into(),
        })
    }
}
