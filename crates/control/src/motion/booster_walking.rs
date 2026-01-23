use color_eyre::{
    eyre::{Context as _, ContextCompat as _},
    Result,
};
use hulkz::Session;
use serde::{Deserialize, Serialize};
use tracing::debug;
use types::{
    cycle_time::CycleTime,
    motion_command::MotionCommand,
    parameters::{MotorCommandParameters, RLWalkingParameters},
};
use walking_inference::inference::WalkingInference;

#[derive(Serialize, Deserialize, Debug)]
struct Parameters {
    prepare_motor_command: MotorCommandParameters,
    common_motor_command: MotorCommandParameters,
    rl_walking: RLWalkingParameters,
}

pub async fn run() -> Result<()> {
    let session = Session::new().await.wrap_err("failed to create session")?;

    let pub_target_joint_positions = session
        .publish("target_joint_positions")
        .await
        .wrap_err("failed to create publisher")?;

    let (imu_state, imu_state_driver) = session
        .buffer("imu_state", 10)
        .await
        .wrap_err("failed to create imu_state buffer")?;
    tokio::spawn(imu_state_driver);

    let (serial_motor_states, serial_motor_states_driver) = session
        .buffer("serial_motor_states", 10)
        .await
        .wrap_err("failed to create serial_motor_states buffer")?;
    tokio::spawn(serial_motor_states_driver);

    let pub_walking_inference_inputs = session.publish("walking_inference_inputs").await?;

    let mut motion_command = session.stream::<MotionCommand>("motion_command").await?;

    let parameters = session.parameters::<Parameters>().await?;
    let neural_network_folder = "hulk/etc/neural_networks/";

    let mut walking_inference = WalkingInference::new(
        neural_network_folder,
        &parameters.get().await.prepare_motor_command,
    )?;

    let mut smoothed_target_joint_positions = parameters
        .get()
        .await
        .prepare_motor_command
        .default_positions;

    let mut last_cycle_start = session.now();

    loop {
        debug!("waiting for motion_command...");
        let motion_command = motion_command
            .recv_async()
            .await
            .wrap_err("failed to receive motion_command")?;
        debug!("received motion_command: {:?}", motion_command);
        let now = &motion_command.timestamp;
        let imu_state = imu_state
            .lookup_nearest(now)
            .wrap_err("failed to get latest imu_state")?;
        let serial_motor_states = serial_motor_states
            .lookup_nearest(now)
            .wrap_err("failed to get latest serial_motor_states")?;
        let cycle_time = CycleTime {
            start_time: now.get_time().to_system_time(),
            last_cycle_duration: now
                .get_time()
                .to_system_time()
                .duration_since(last_cycle_start.get_time().to_system_time())
                .expect("Time ran backwards"),
        };

        let (walking_inference_inputs, inference_output_positions) = walking_inference
            .do_inference(
                cycle_time,
                &motion_command.payload,
                &imu_state.payload,
                serial_motor_states.payload,
                &parameters.get().await.rl_walking,
                &parameters.get().await.common_motor_command,
            )?;

        pub_walking_inference_inputs
            .put_with_subscription(|| walking_inference_inputs.clone())
            .await?;

        let target_joint_positions = parameters
            .get()
            .await
            .common_motor_command
            .default_positions
            + inference_output_positions * parameters.get().await.rl_walking.control.action_scale;

        smoothed_target_joint_positions = smoothed_target_joint_positions
            * parameters
                .get()
                .await
                .rl_walking
                .joint_position_smoothing_factor
            + target_joint_positions
                * (1.0
                    - parameters
                        .get()
                        .await
                        .rl_walking
                        .joint_position_smoothing_factor);

        pub_target_joint_positions
            .put(&smoothed_target_joint_positions)
            .await?;

        last_cycle_start = *now;
    }
}
