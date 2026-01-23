use booster::{LowState, MotorState};
use color_eyre::{eyre::WrapErr, Result};
use hulkz::Session;
use tracing::debug;
use types::{
    joints::{arm::ArmJoints, head::HeadJoints, leg::LegJoints, Joints},
    sensor_data::SensorData,
};

#[tracing::instrument]
pub async fn run() -> Result<()> {
    let session = Session::new().await.wrap_err("failed to create session")?;

    let pub_imu_state = session
        .publish("imu_state")
        .await
        .wrap_err("failed to create imu_state publisher")?;
    let pub_serial_motor_states = session
        .publish("serial_motor_states")
        .await
        .wrap_err("failed to create serial_motor_states publisher")?;
    let pub_sensor_data = session
        .publish("sensor_data")
        .await
        .wrap_err("failed to create sensor_data publisher")?;

    let mut low_state = session
        .stream("booster/low_state")
        .await
        .wrap_err("failed to create low_state stream")?;

    loop {
        debug!("waiting for low_state...");
        let low_state: LowState = low_state
            .recv_async()
            .await
            .wrap_err("failed to receive low_state")?
            .payload;
        debug!("received low_state");

        let positions = Joints {
            head: HeadJoints {
                yaw: low_state.motor_state_serial[0].position,
                pitch: low_state.motor_state_serial[1].position,
            },
            left_arm: ArmJoints {
                shoulder_pitch: low_state.motor_state_serial[2].position,
                shoulder_roll: low_state.motor_state_serial[3].position,
                shoulder_yaw: low_state.motor_state_serial[4].position,
                elbow: low_state.motor_state_serial[5].position,
            },
            right_arm: ArmJoints {
                shoulder_pitch: low_state.motor_state_serial[6].position,
                shoulder_roll: low_state.motor_state_serial[7].position,
                shoulder_yaw: low_state.motor_state_serial[8].position,
                elbow: low_state.motor_state_serial[9].position,
            },
            left_leg: LegJoints {
                hip_pitch: low_state.motor_state_serial[10].position,
                hip_roll: low_state.motor_state_serial[11].position,
                hip_yaw: low_state.motor_state_serial[12].position,
                knee: low_state.motor_state_serial[13].position,
                ankle_up: low_state.motor_state_serial[14].position,
                ankle_down: low_state.motor_state_serial[15].position,
            },
            right_leg: LegJoints {
                hip_pitch: low_state.motor_state_serial[16].position,
                hip_roll: low_state.motor_state_serial[17].position,
                hip_yaw: low_state.motor_state_serial[18].position,
                knee: low_state.motor_state_serial[19].position,
                ankle_up: low_state.motor_state_serial[20].position,
                ankle_down: low_state.motor_state_serial[21].position,
            },
        };

        let sensor_data = SensorData {
            positions,
            ..SensorData::default()
        };

        let serial_motor_states = low_state
            .motor_state_serial
            .into_iter()
            .collect::<Joints<MotorState>>();

        pub_imu_state
            .put(&low_state.imu_state)
            .await
            .wrap_err("failed to publish imu_state")?;
        pub_serial_motor_states
            .put(&serial_motor_states)
            .await
            .wrap_err("failed to publish serial_motor_states")?;
        pub_sensor_data
            .put(&sensor_data)
            .await
            .wrap_err("failed to publish sensor_data")?;
    }
}
