use byteorder::{ByteOrder, LittleEndian};
use module_derive::module;
use simple_websockets::{Event, EventHub, Responder};
use std::collections::HashMap;
use types::{FallState, ForceSensitiveResistors, InertialMeasurementUnitData, Joints, SensorData};

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

const ACTION_SIZE: usize = 2 * 26;
const OBSERVATION_SIZE: usize = 3 * 26 + 2 * 8 + 2;

pub struct StabilizationInterface {
    position_offsets: Joints,
    stiffness_offsets: Joints,
    event_hub: EventHub,
    clients: HashMap<u64, Responder>,
}

#[module(control)]
#[input(path = fall_state, data_type = FallState, required)]
#[input(path = positions, data_type = Joints, required)]
#[input(path = sensor_data, data_type = SensorData, required)]
#[input(path = step_t, data_type = f32, required)]
#[input(path = stiffnesses, data_type = Joints, required)]
#[input(path = unstable_score, data_type = usize, required)]
#[main_output(name = position_offsets, data_type = Joints)]
#[main_output(name = stiffness_offsets, data_type = Joints)]
impl StabilizationInterface {}

impl StabilizationInterface {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            position_offsets: Joints::default(),
            stiffness_offsets: Joints::default(),
            event_hub: simple_websockets::launch(9990).expect("failed to listen on port 9990"),
            clients: HashMap::new(),
        })
    }

    fn compose_observation(&self, context: &CycleContext) -> [f32; OBSERVATION_SIZE] {
        let sensor_data = context.sensor_data.clone();
        let positions = context.positions.clone();
        let stiffnesses = context.stiffnesses.clone();
        let stability = match context.fall_state {
            FallState::Upright => 1.0 - (0.3 * *context.unstable_score as f32),
            _ => 0.0,
        };
        let step_t = if context.step_t.is_nan() {0.0} else {*context.step_t};
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

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
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
                    let (positions, stiffnesses) = action.split_at(ACTION_SIZE / 2);

                    // apply action
                    self.position_offsets = Joints::from_angles(
                        positions.try_into().expect("slice with incorrect length"),
                    );
                    self.stiffness_offsets = Joints::from_angles(
                        stiffnesses.try_into().expect("slice with incorrect length"),
                    );

                    // respond with observation
                    let responder = self.clients.get(&client_id).unwrap();
                    let mut bytes = [0; OBSERVATION_SIZE * 4];
                    let observation = self.compose_observation(&context);
                    LittleEndian::write_f32_into(&observation, &mut bytes);
                    responder.send(simple_websockets::Message::Binary(bytes.into()));
                }
            }
        }

        Ok(MainOutputs {
            position_offsets: Some(self.position_offsets),
            stiffness_offsets: Some(self.stiffness_offsets),
        })
    }
}
