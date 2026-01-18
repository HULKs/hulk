use booster::LowState;
use color_eyre::{eyre::Context as _, Result};
use hulkz::Session;
use types::{
    joints::{arm::ArmJoints, head::HeadJoints, leg::LegJoints, Joints},
    sensor_data::SensorData,
};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let session = Session::new().await?;

    let mut stream = session.stream::<LowState>("booster/low_state").await?;

    let publisher = session
        .publish("sensor_data")
        .await
        .wrap_err("failed to create publisher")?;

    loop {
        match stream.recv_async().await {
            Ok(low_state) => {
                // let now = context.hardware_interface.get_now();
                // let cycle_time = CycleTime {
                //     start_time: now,
                //     last_cycle_duration: now
                //         .duration_since(self.last_cycle_start)
                //         .expect("time ran backwards"),
                // };
                // self.last_cycle_start = now;
                dbg!(&low_state);
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
                publisher
                    .put(&sensor_data)
                    .await
                    .wrap_err("failed to put sensor_data to the publisher")?;
            }
            Err(err) => {
                tracing::error!("Failed to receive LowState: {err}");
            }
        }
    }
}
