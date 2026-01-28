use color_eyre::{
    eyre::{Context as _, ContextCompat as _},
    Result,
};
use hulkz::{Buffer, Session};
use tracing::debug;
use types::{
    cycle_time::CycleTime,
    motion_command::MotionCommand,
    parameters::{MotorCommandParameters, RLWalkingParameters},
};
use walking_inference::inference::WalkingInference;

pub async fn run() -> Result<()> {
    let namespace = "HULK10";
    let session = Session::create(namespace)
        .await
        .wrap_err("failed to create session")?;

    let node = session
        .create_node("booster_walking")
        .build()
        .await
        .wrap_err("failed to create node")?;

    let pub_target_joint_positions = node
        .create_publisher("target_joint_positions")
        .build()
        .await
        .wrap_err("failed to create publisher")?;

    let (imu_state, imu_state_driver) = Buffer::new(
        node.create_subscriber("imu_state")
            .build()
            .await
            .wrap_err("failed to create imu_state subscriber")?,
        10,
    );
    tokio::spawn(imu_state_driver);

    let (serial_motor_states, serial_motor_states_driver) = Buffer::new(
        node.create_subscriber("serial_motor_states")
            .build()
            .await
            .wrap_err("failed to create serial_motor_states subscriber")?,
        10,
    );
    tokio::spawn(serial_motor_states_driver);

    let pub_walking_inference_inputs = node
        .create_publisher("walking_inference_inputs")
        .build()
        .await
        .wrap_err("failed to create walking_inference_inputs publisher")?;

    let mut motion_command = node
        .create_subscriber::<MotionCommand>("motion_command")
        .build()
        .await
        .wrap_err("failed to create motion_command subscriber")?;

    let (prepare_motor_command, prepare_motor_command_driver) = node
        .declare_parameter::<MotorCommandParameters>("prepare_motor_command")
        .build()
        .await
        .wrap_err("failed to declare prepare_motor_command parameter")?;
    tokio::spawn(prepare_motor_command_driver);
    let (common_motor_command, common_motor_command_driver) = node
        .declare_parameter::<MotorCommandParameters>("common_motor_command")
        .build()
        .await
        .wrap_err("failed to declare common_motor_command parameter")?;
    tokio::spawn(common_motor_command_driver);
    let (rl_walking, rl_walking_driver) = node
        .declare_parameter::<RLWalkingParameters>("rl_walking")
        .build()
        .await
        .wrap_err("failed to declare rl_walking parameter")?;
    tokio::spawn(rl_walking_driver);

    let neural_network_folder = "etc/neural_networks/";
    let mut walking_inference =
        WalkingInference::new(neural_network_folder, &*prepare_motor_command.get().await)?;

    let mut smoothed_target_joint_positions = prepare_motor_command.get().await.default_positions;

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
            .await
            .wrap_err("failed to get latest imu_state")?;
        let serial_motor_states = serial_motor_states
            .lookup_nearest(now)
            .await
            .wrap_err("failed to get latest serial_motor_states")?;
        let cycle_time = CycleTime {
            start_time: now.get_time().to_system_time(),
            last_cycle_duration: now
                .get_time()
                .to_system_time()
                .duration_since(last_cycle_start.get_time().to_system_time())
                .expect("Time ran backwards"),
        };

        let rl_walking = &*rl_walking.get().await;
        let common_motor_command = &*common_motor_command.get().await;
        let (walking_inference_inputs, inference_output_positions) = walking_inference
            .do_inference(
                cycle_time,
                &motion_command.payload,
                &imu_state.payload,
                serial_motor_states.payload,
                rl_walking,
                common_motor_command,
            )?;

        pub_walking_inference_inputs
            .put_if_subscribed(|| walking_inference_inputs.clone())
            .await?;

        let target_joint_positions = common_motor_command.default_positions
            + inference_output_positions * rl_walking.control.action_scale;

        smoothed_target_joint_positions = smoothed_target_joint_positions
            * rl_walking.joint_position_smoothing_factor
            + target_joint_positions * (1.0 - rl_walking.joint_position_smoothing_factor);

        pub_target_joint_positions
            .put(&smoothed_target_joint_positions)
            .await?;

        last_cycle_start = *now;
    }
}
