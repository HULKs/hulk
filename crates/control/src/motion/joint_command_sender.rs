use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{hardware::Interface, HeadJoints, Joints, Leds, Rgb, SensorData};

pub struct JointCommandSender {
    last_average_color: Rgb,
}

#[context]
pub struct NewContext {
    pub center_head_position: Parameter<HeadJoints, "control/center_head_position">,
    pub penalized_pose: Parameter<Joints, "control/penalized_pose">,
    pub ready_pose: Parameter<Joints, "control/ready_pose">,
}

#[context]
pub struct CycleContext {
    pub center_head_position: Parameter<HeadJoints, "control/center_head_position">,
    pub penalized_pose: Parameter<Joints, "control/penalized_pose">,
    pub ready_pose: Parameter<Joints, "control/ready_pose">,

    // pub arms_up_squat_joints_command:
    //     RequiredInput<Option<JointsCommand>, "arms_up_squat_joints_command?">,
    // pub dispatching_command: RequiredInput<Option<JointsCommand>, "dispatching_command?">,
    // pub fall_protection_command: RequiredInput<Option<JointsCommand>, "fall_protection_command?">,
    // pub head_joints_command: RequiredInput<Option<HeadJointsCommand>, "head_joints_command?">,
    // pub jump_left_joints_command: RequiredInput<Option<JointsCommand>, "jump_left_joints_command?">,
    // pub jump_right_joints_command:
    //     RequiredInput<Option<JointsCommand>, "jump_right_joints_command?">,
    // pub motion_selection: RequiredInput<Option<MotionSelection>, "motion_selection?">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    // pub sit_down_joints_command: RequiredInput<Option<JointsCommand>, "sit_down_joints_command?">,
    // pub stand_up_back_positions: RequiredInput<Option<Joints>, "stand_up_back_positions?">,
    // pub stand_up_front_positions: RequiredInput<Option<Joints>, "stand_up_front_positions?">,
    // pub walk_joints_command: RequiredInput<Option<BodyJointsCommand>, "walk_joints_command?">,
    pub hardware_interface: HardwareInterface,
    pub average_color: PerceptionInput<Rgb, "VisionTop", "average_color">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub positions: MainOutput<Option<Joints>>,
    pub stiffnesses: MainOutput<Option<Joints>>,
}

impl JointCommandSender {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {
            last_average_color: Default::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        if let Some(color) = context
            .average_color
            .persistent
            .values()
            .rev()
            .find_map(|datas| datas.last())
        {
            self.last_average_color = **color;
        }
        context
            .hardware_interface
            .write_to_actuators(
                Joints::default(),
                Joints::default(),
                Leds {
                    left_ear: 0.0.into(),
                    right_ear: 0.0.into(),
                    chest: self.last_average_color,
                    left_foot: self.last_average_color,
                    right_foot: self.last_average_color,
                    left_eye: self.last_average_color.into(),
                    right_eye: self.last_average_color.into(),
                },
            )
            .wrap_err("failed to write to actuators")?;
        Ok(MainOutputs::default())
    }
}
