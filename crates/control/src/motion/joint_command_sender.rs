use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};
use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::AdditionalOutput;
use simple_websockets::{Event, EventHub, Responder};
use types::{
    hardware::Interface, BodyJointsCommand, FallState, ForceSensitiveResistors, HeadJoints,
    HeadJointsCommand, InertialMeasurementUnitData, Joints, JointsCommand, Leds, MotionSelection,
    MotionType, SensorData,
};

pub struct JointCommandSender {
    position_offsets: Joints,
    stiffness_offsets: Joints,
    event_hub: EventHub,
    clients: HashMap<u64, Responder>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub positions: AdditionalOutput<Joints<f32>, "positions">,
    pub positions_difference: AdditionalOutput<Joints<f32>, "positions_difference">,
    pub stiffnesses: AdditionalOutput<Joints<f32>, "stiffnesses">,

    pub center_head_position: Parameter<HeadJoints<f32>, "center_head_position">,
    pub penalized_pose: Parameter<Joints<f32>, "penalized_pose">,
    pub ready_pose: Parameter<Joints<f32>, "ready_pose">,

    pub arms_up_squat_joints_command: Input<JointsCommand<f32>, "arms_up_squat_joints_command">,
    pub dispatching_command: Input<JointsCommand<f32>, "dispatching_command">,
    pub fall_protection_command: Input<JointsCommand<f32>, "fall_protection_command">,
    pub head_joints_command: Input<HeadJointsCommand<f32>, "head_joints_command">,
    pub jump_left_joints_command: Input<JointsCommand<f32>, "jump_left_joints_command">,
    pub jump_right_joints_command: Input<JointsCommand<f32>, "jump_right_joints_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub sit_down_joints_command: Input<JointsCommand<f32>, "sit_down_joints_command">,
    pub stand_up_back_positions: Input<Joints<f32>, "stand_up_back_positions">,
    pub stand_up_front_positions: Input<Joints<f32>, "stand_up_front_positions">,
    pub walk_joints_command: Input<BodyJointsCommand<f32>, "walk_joints_command">,
    pub hardware_interface: HardwareInterface,
    pub leds: Input<Leds, "leds">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl JointCommandSender {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            position_offsets: Joints::default(),
            stiffness_offsets: Joints::default(),
            event_hub: simple_websockets::launch(9990).expect("failed to listen on port 9990"),
            clients: HashMap::new(),
        })
    }

    fn compose_observation(
        &self,
        positions: &Joints,
        stiffnesses: &Joints,
        context: &CycleContext<impl Interface>,
    ) -> [f32; OBSERVATION_SIZE] {
        let sensor_data = context.sensor_data.clone();
        let stability = match context.fall_state {
            FallState::Upright => 1.0 - (0.3 * *context.unstable_score as f32),
            _ => 0.0,
        };
        let step_t = if context.step_t.is_nan() {
            0.0
        } else {
            *context.step_t
        };
        <[f32; OBSERVATION_SIZE]>::try_from(
            [
                [stability, step_t].as_slice(),
                sensor_data.positions.to_angles().as_slice(),
                positions.to_angles().as_slice(),
                stiffnesses.to_angles().as_slice(),
                flat_imu(&sensor_data.inertial_measurement_unit).as_slice(),
                flat_fsr(&sensor_data.force_sensitive_resistors).as_slice(),
            ]
            .concat(),
        )
        .unwrap()
    }

    pub fn cycle(&mut self, mut context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let current_positions = context.sensor_data.positions;
        let dispatching_command = context.dispatching_command;
        let fall_protection_positions = context.fall_protection_command.positions;
        let fall_protection_stiffnesses = context.fall_protection_command.stiffnesses;
        let head_joints_command = context.head_joints_command;
        let motion_selection = context.motion_selection;
        let arms_up_squat = context.arms_up_squat_joints_command;
        let jump_left = context.jump_left_joints_command;
        let jump_right = context.jump_right_joints_command;
        let sit_down = context.sit_down_joints_command;
        let stand_up_back_positions = context.stand_up_back_positions;
        let stand_up_front_positions = context.stand_up_front_positions;
        let walk = context.walk_joints_command;

        let (positions, stiffnesses) = match motion_selection.current_motion {
            MotionType::ArmsUpSquat => (arms_up_squat.positions, arms_up_squat.stiffnesses),
            MotionType::Dispatching => (
                dispatching_command.positions,
                dispatching_command.stiffnesses,
            ),
            MotionType::FallProtection => (fall_protection_positions, fall_protection_stiffnesses),
            MotionType::JumpLeft => (jump_left.positions, jump_left.stiffnesses),
            MotionType::JumpRight => (jump_right.positions, jump_right.stiffnesses),
            MotionType::Penalized => (*context.penalized_pose, Joints::fill(0.8)),
            MotionType::SitDown => (sit_down.positions, sit_down.stiffnesses),
            MotionType::Stand => (
                Joints::from_head_and_body(head_joints_command.positions, walk.positions),
                Joints::from_head_and_body(head_joints_command.stiffnesses, walk.stiffnesses),
            ),
            MotionType::StandUpBack => (*stand_up_back_positions, Joints::fill(1.0)),
            MotionType::StandUpFront => (*stand_up_front_positions, Joints::fill(1.0)),
            MotionType::Unstiff => (current_positions, Joints::fill(0.0)),
            MotionType::Walk => (
                Joints::from_head_and_body(head_joints_command.positions, walk.positions),
                Joints::from_head_and_body(head_joints_command.stiffnesses, walk.stiffnesses),
            ),
        };

        while !self.event_hub.is_empty() {
            match self.event_hub.poll_event() {
                Event::Connect(client_id, responder) => {
                    self.clients.insert(client_id, responder);
                }
                Event::Disconnect(client_id) => {
                    self.clients.remove(&client_id);
                }
                Event::Message(client_id, message) => {
                    // read action
                    let bytes: Vec<u8> = match message {
                        simple_websockets::Message::Text(_) => todo!(),
                        simple_websockets::Message::Binary(bin) => bin,
                    };
                    let mut action = [0.0; ACTION_SIZE];
                    LittleEndian::read_f32_into(&bytes, &mut action);
                    let (position_offsets, stiffness_offsets) = action.split_at(ACTION_SIZE / 2);

                    // apply action
                    self.position_offsets = Joints::from_angles(
                        position_offsets
                            .try_into()
                            .expect("slice with incorrect length"),
                    );
                    self.stiffness_offsets = Joints::from_angles(
                        stiffness_offsets
                            .try_into()
                            .expect("slice with incorrect length"),
                    );

                    // respond with observation
                    let responder = self.clients.get(&client_id).unwrap();
                    let mut bytes = [0; OBSERVATION_SIZE * 4];
                    let observation = self.compose_observation(&current_positions, &stiffnesses, &context);
                    LittleEndian::write_f32_into(&observation, &mut bytes);
                    responder.send(simple_websockets::Message::Binary(bytes.into()));
                }
            }
        }

        context
            .hardware_interface
            .write_to_actuators(
                positions + self.position_offsets,
                stiffnesses + self.stiffness_offsets,
                *context.leds,
            )
            .wrap_err("failed to write to actuators")?;

        context.positions.fill_if_subscribed(|| positions);
        context
            .positions_difference
            .fill_if_subscribed(|| positions - current_positions);
        context
            .position_offsets
            .fill_if_subscribed(|| self.position_offsets);
        context.stiffnesses.fill_if_subscribed(|| stiffnesses);
        context
            .stiffness_offsets
            .fill_if_subscribed(|| self.stiffness_offsets);

        Ok(MainOutputs {})
    }
}

const ACTION_SIZE: usize = 2 * 26;
const OBSERVATION_SIZE: usize = 3 * 26 + 2 * 8 + 2;

fn flat_imu(imu: &InertialMeasurementUnitData) -> [f32; 8] {
    [
        imu.linear_acceleration[0],
        imu.linear_acceleration[1],
        imu.linear_acceleration[2],
        imu.angular_velocity[0],
        imu.angular_velocity[1],
        imu.angular_velocity[2],
        imu.roll_pitch[0],
        imu.roll_pitch[1],
    ]
}

fn flat_fsr(fsr: &ForceSensitiveResistors) -> [f32; 8] {
    [
        fsr.left.front_left,
        fsr.left.front_right,
        fsr.left.rear_left,
        fsr.left.rear_right,
        fsr.right.front_left,
        fsr.right.front_right,
        fsr.right.rear_left,
        fsr.right.rear_right,
    ]
}
