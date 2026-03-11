use std::{
    env,
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::SystemTime,
};

use booster::{
    ButtonEventMsg, FallDownState, Kick, LowCommand, LowState, Odometer, RemoteControllerState,
};
use booster_sdk::{client::BoosterClient, types::RobotMode};
use cdr::{CdrLe, Infinite};
use color_eyre::{
    Result,
    eyre::{Context as _, ContextCompat as _, bail, eyre},
};
use hardware::{
    ButtonEventMsgInterface, CameraInterface, FallDownStateInterface, HighLevelInterface,
    IdInterface, LowCommandInterface, LowStateInterface, MicrophoneInterface,
    MotionRuntimeInteface, NetworkInterface, OdometerInterface, PathsInterface, RecordingInterface,
    RemoteControllerStateInterface, SimulatorInterface, SpeakerInterface, TimeInterface,
    VisualKickInterface,
};
use hsl_network::endpoint::{Endpoint, Ports};
use hula_types::hardware::{Ids, Paths};
use kinematics::joints::head::HeadJoints;
use log::{debug, error, warn};
use parking_lot::Mutex;
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use serde::{Deserialize, de::DeserializeOwned};
use tokio::{
    runtime::Handle,
    sync::mpsc::{self, Receiver, Sender},
};
use tokio_util::sync::CancellationToken;
use types::{
    audio::SpeakerRequest,
    messages::{IncomingMessage, OutgoingMessage},
    motion_runtime::MotionRuntime,
    samples::Samples,
    step::Step,
};

use crate::{
    HardwareInterface,
    latest_receiver::{LatestReceiver, LatestSender, latest_channel},
};
use zenoh::{
    Session,
    handlers::{RingChannel, RingChannelHandler},
    sample::Sample,
};

const COMMAND_CHANNEL_CAPACITY: usize = 10;
const ZENOH_LOCALHOST_ENDPOINT: &str = "tcp/127.0.0.1:7447";
const LOW_STATE_TOPIC: &str = "rt/low_state";
const JOINT_CTRL_TOPIC: &str = "rt/joint_ctrl"; // not needed
const KICK_BALL_TOPIC: &str = "rt/kick_ball";
const ODOMETER_STATE_TOPIC: &str = "rt/odometer_state";
const FALL_DOWN_TOPIC: &str = "rt/fall_down";
const BUTTON_EVENT_TOPIC: &str = "rt/button_event";
const REMOTE_CONTROLLER_STATE_TOPIC: &str = "rt/remote_controller_state";
const RECTIFIED_IMAGE_TOPIC: &str = "rt/StereoNetNode/rectified_image"; // not needed
const STEREONET_DEPTH_TOPIC: &str = "rt/StereoNetNode/stereonet_depth"; // not needed
const STEREONET_DEPTH_CAMERA_INFO_TOPIC: &str = "rt/StereoNetNode/stereonet_depth/camera_info"; // no needed
const IMAGE_LEFT_RAW_TOPIC: &str = "rt/image_left_raw";
const IMAGE_LEFT_RAW_CAMERA_INFO_TOPIC: &str = "rt/image_left_raw/camera_info";

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub hsl_network_ports: Ports,
    pub paths: Paths,
}

struct ZenohBackendHandles {
    low_state_receiver: LatestReceiver<LowState>,
    joint_control_sender: Sender<LowCommand>,
    kick_ball_sender: Sender<Kick>,
    odometer_receiver: LatestReceiver<Odometer>,
    fall_down_state_receiver: LatestReceiver<FallDownState>,
    button_event_msg_receiver: LatestReceiver<ButtonEventMsg>,
    remote_controller_state_receiver: LatestReceiver<RemoteControllerState>,
    rectified_image_receiver: LatestReceiver<Image>,
    stereonet_depth_receiver: LatestReceiver<Image>,
    stereonet_depth_camera_info_receiver: LatestReceiver<CameraInfo>,
    image_left_raw_receiver: LatestReceiver<Image>,
    image_left_raw_camera_info_receiver: LatestReceiver<CameraInfo>,
}

pub struct BoosterHardwareInterface {
    ids: Ids,
    paths: Paths,
    enable_recording: AtomicBool,

    low_state_receiver: Mutex<LatestReceiver<LowState>>,
    joint_control_sender: Sender<LowCommand>,
    kick_ball_sender: Sender<Kick>,
    odometer_receiver: Mutex<LatestReceiver<Odometer>>,
    fall_down_state_receiver: Mutex<LatestReceiver<FallDownState>>,
    button_event_msg_receiver: Mutex<LatestReceiver<ButtonEventMsg>>,
    remote_controller_state_receiver: Mutex<LatestReceiver<RemoteControllerState>>,
    rectified_image_receiver: Mutex<LatestReceiver<Image>>,
    stereonet_depth_receiver: Mutex<LatestReceiver<Image>>,
    stereonet_depth_camera_info_receiver: Mutex<LatestReceiver<CameraInfo>>,
    image_left_raw_receiver: Mutex<LatestReceiver<Image>>,
    image_left_raw_camera_info_receiver: Mutex<LatestReceiver<CameraInfo>>,

    high_level_interface_client: Arc<BoosterClient>,
    robot_mode: Arc<Mutex<RobotMode>>,

    runtime_handle: Handle,
    hsl_network_endpoint: Endpoint,
    keep_running: CancellationToken,
}

impl BoosterHardwareInterface {
    pub async fn new(
        runtime_handle: Handle,
        keep_running: CancellationToken,
        parameters: Parameters,
    ) -> Result<Self> {
        let zenoh_backend = initialize_zenoh_backend(keep_running.clone())
            .await
            .wrap_err("failed to initialize Zenoh backend")?;

        let Some(hardware_id) = env::var_os("HARDWARE_ID") else {
            bail!("environment variable HARDWARE_ID not set")
        };
        let ids = Ids {
            robot_id: hardware_id
                .into_string()
                .ok()
                .wrap_err("id was not valid UTF-8")?,
        };

        let high_level_interface_client = Arc::new(BoosterClient::new()?);
        let robot_mode = Arc::new(Mutex::new(RobotMode::Unknown));

        tokio::spawn(
            keep_running
                .clone()
                .run_until_cancelled_owned(robot_mode_worker(
                    keep_running.clone(),
                    high_level_interface_client.clone(),
                    robot_mode.clone(),
                )),
        );

        Ok(Self {
            ids,
            paths: parameters.paths,
            enable_recording: AtomicBool::new(false),

            low_state_receiver: Mutex::new(zenoh_backend.low_state_receiver),
            joint_control_sender: zenoh_backend.joint_control_sender,
            kick_ball_sender: zenoh_backend.kick_ball_sender,
            odometer_receiver: Mutex::new(zenoh_backend.odometer_receiver),
            fall_down_state_receiver: Mutex::new(zenoh_backend.fall_down_state_receiver),
            button_event_msg_receiver: Mutex::new(zenoh_backend.button_event_msg_receiver),
            remote_controller_state_receiver: Mutex::new(
                zenoh_backend.remote_controller_state_receiver,
            ),
            rectified_image_receiver: Mutex::new(zenoh_backend.rectified_image_receiver),
            stereonet_depth_receiver: Mutex::new(zenoh_backend.stereonet_depth_receiver),
            stereonet_depth_camera_info_receiver: Mutex::new(
                zenoh_backend.stereonet_depth_camera_info_receiver,
            ),
            image_left_raw_receiver: Mutex::new(zenoh_backend.image_left_raw_receiver),
            image_left_raw_camera_info_receiver: Mutex::new(
                zenoh_backend.image_left_raw_camera_info_receiver,
            ),

            robot_mode,
            high_level_interface_client,

            hsl_network_endpoint: keep_running
                .clone()
                .run_until_cancelled(Endpoint::new(parameters.hsl_network_ports))
                .await
                .ok_or(eyre!("termination requested"))?
                .wrap_err("failed to initialize HSL network")?,
            runtime_handle,
            keep_running,
        })
    }

    fn run_until_cancelled<T>(&self, fut: impl Future<Output = T>) -> Result<T> {
        self.runtime_handle
            .clone()
            .block_on(self.keep_running.run_until_cancelled(fut))
            .ok_or(eyre!("termination_requested"))
    }
}

async fn initialize_zenoh_backend(keep_running: CancellationToken) -> Result<ZenohBackendHandles> {
    let zenoh_session = zenoh::open(localhost_zenoh_config()?)
        .await
        .map_err(|error| eyre!("failed to create Zenoh session: {error}"))?;

    let low_state_receiver = spawn_subscription_worker::<LowState>(
        zenoh_session.clone(),
        keep_running.clone(),
        LOW_STATE_TOPIC,
    )
    .await?;
    let joint_control_sender = spawn_publisher_worker::<LowCommand>(
        zenoh_session.clone(),
        keep_running.clone(),
        JOINT_CTRL_TOPIC,
        COMMAND_CHANNEL_CAPACITY,
    )
    .await?;
    let kick_ball_sender = spawn_publisher_worker::<Kick>(
        zenoh_session.clone(),
        keep_running.clone(),
        KICK_BALL_TOPIC,
        COMMAND_CHANNEL_CAPACITY,
    )
    .await?;
    let odometer_receiver = spawn_subscription_worker::<Odometer>(
        zenoh_session.clone(),
        keep_running.clone(),
        ODOMETER_STATE_TOPIC,
    )
    .await?;
    let fall_down_state_receiver = spawn_subscription_worker::<FallDownState>(
        zenoh_session.clone(),
        keep_running.clone(),
        FALL_DOWN_TOPIC,
    )
    .await?;
    let button_event_msg_receiver = spawn_subscription_worker::<ButtonEventMsg>(
        zenoh_session.clone(),
        keep_running.clone(),
        BUTTON_EVENT_TOPIC,
    )
    .await?;
    let remote_controller_state_receiver = spawn_subscription_worker::<RemoteControllerState>(
        zenoh_session.clone(),
        keep_running.clone(),
        REMOTE_CONTROLLER_STATE_TOPIC,
    )
    .await?;
    let rectified_image_receiver = spawn_subscription_worker::<Image>(
        zenoh_session.clone(),
        keep_running.clone(),
        RECTIFIED_IMAGE_TOPIC,
    )
    .await?;
    let stereonet_depth_receiver = spawn_subscription_worker::<Image>(
        zenoh_session.clone(),
        keep_running.clone(),
        STEREONET_DEPTH_TOPIC,
    )
    .await?;
    let stereonet_depth_camera_info_receiver = spawn_subscription_worker::<CameraInfo>(
        zenoh_session.clone(),
        keep_running.clone(),
        STEREONET_DEPTH_CAMERA_INFO_TOPIC,
    )
    .await?;
    let image_left_raw_receiver = spawn_subscription_worker::<Image>(
        zenoh_session.clone(),
        keep_running.clone(),
        IMAGE_LEFT_RAW_TOPIC,
    )
    .await?;
    let image_left_raw_camera_info_receiver = spawn_subscription_worker::<CameraInfo>(
        zenoh_session.clone(),
        keep_running.clone(),
        IMAGE_LEFT_RAW_CAMERA_INFO_TOPIC,
    )
    .await?;

    tokio::spawn(async move {
        let _zenoh_session = zenoh_session;
        keep_running.cancelled().await;
    });

    Ok(ZenohBackendHandles {
        low_state_receiver,
        joint_control_sender,
        kick_ball_sender,
        odometer_receiver,
        fall_down_state_receiver,
        button_event_msg_receiver,
        remote_controller_state_receiver,
        rectified_image_receiver,
        stereonet_depth_receiver,
        stereonet_depth_camera_info_receiver,
        image_left_raw_receiver,
        image_left_raw_camera_info_receiver,
    })
}

fn localhost_zenoh_config() -> Result<zenoh::Config> {
    let mut config = zenoh::Config::default();
    config
        .insert_json5("mode", r#""client""#)
        .map_err(|error| eyre!("failed to set Zenoh mode: {error}"))?;
    config
        .insert_json5(
            "connect/endpoints",
            &format!(r#"["{ZENOH_LOCALHOST_ENDPOINT}"]"#),
        )
        .map_err(|error| eyre!("failed to set Zenoh connect endpoint: {error}"))?;
    // config
    //     .insert_json5("listen/endpoints", "[]")
    //     .map_err(|error| eyre!("failed to disable Zenoh listeners: {error}"))?;
    // config
    //     .insert_json5("scouting/multicast/enabled", "false")
    //     .map_err(|error| eyre!("failed to disable Zenoh multicast scouting: {error}"))?;
    // config
    //     .insert_json5("scouting/gossip/enabled", "false")
    //     .map_err(|error| eyre!("failed to disable Zenoh gossip scouting: {error}"))?;
    Ok(config)
}

async fn spawn_subscription_worker<T: DeserializeOwned + Send + Sync + 'static>(
    zenoh_session: Session,
    keep_running: CancellationToken,
    key_expression: &'static str,
) -> Result<LatestReceiver<T>> {
    let subscriber = zenoh_session
        .declare_subscriber(key_expression)
        .with(RingChannel::new(10))
        .await
        .map_err(|error| {
            eyre!("failed to create Zenoh subscriber for `{key_expression}`: {error}")
        })?;
    let (sender, receiver) = latest_channel(key_expression);

    let task_name = key_expression;
    spawn_monitored_task(
        keep_running.clone(),
        task_name,
        forward_subscription(subscriber, sender, keep_running),
    );
    Ok(receiver)
}

async fn spawn_publisher_worker<T: Send + serde::Serialize + 'static>(
    zenoh_session: Session,
    keep_running: CancellationToken,
    key_expression: &'static str,
    channel_capacity: usize,
) -> Result<Sender<T>> {
    let publisher = zenoh_session
        .declare_publisher(key_expression)
        .await
        .map_err(|error| {
            eyre!("failed to create Zenoh publisher for `{key_expression}`: {error}")
        })?;
    let (sender, receiver) = mpsc::channel(channel_capacity);

    let task_name = key_expression;
    spawn_monitored_task(
        keep_running.clone(),
        task_name,
        forward_publications(publisher, receiver, keep_running),
    );

    Ok(sender)
}

fn spawn_monitored_task<F>(keep_running: CancellationToken, task_name: &'static str, fut: F)
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        if let Err(error) = fut.await {
            error!("Zenoh backend task `{task_name}` failed: {error:?}");
            keep_running.cancel();
        }
    });
}

async fn forward_subscription<T: DeserializeOwned + Send + Sync + 'static>(
    subscriber: zenoh::pubsub::Subscriber<RingChannelHandler<Sample>>,
    sender: LatestSender<T>,
    keep_running: CancellationToken,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = keep_running.cancelled() => return Ok(()),
            sample = subscriber.recv_async() => {
                let sample = sample.map_err(|error| eyre!("failed to receive Zenoh sample: {error}"))?;
                let payload = sample.payload().to_bytes();
                let message = deserialize_zenoh_message::<T>(payload.as_ref())
                    .wrap_err("failed to decode Zenoh payload")?;
                // info!("received new message on Zenoh topic `{}`", sample.key_expr());
                sender.send_latest(message);
            }
        }
    }
}

async fn forward_publications<T: Send + serde::Serialize + 'static>(
    publisher: zenoh::pubsub::Publisher<'static>,
    mut receiver: Receiver<T>,
    keep_running: CancellationToken,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = keep_running.cancelled() => return Ok(()),
            maybe_message = receiver.recv() => {
                let Some(message) = maybe_message else {
                    return Ok(());
                };
                let payload = serialize_zenoh_message(&message)
                    .wrap_err("failed to encode Zenoh payload")?;
                publisher
                    .put(payload)
                    .await
                    .map_err(|error| eyre!("failed to publish Zenoh message: {error}"))?;
            }
        }
    }
}

fn serialize_zenoh_message<T: serde::Serialize>(message: &T) -> Result<Vec<u8>> {
    cdr::serialize::<_, _, CdrLe>(message, Infinite).wrap_err("failed to serialize CDR payload")
}

fn deserialize_zenoh_message<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    cdr::deserialize(bytes).wrap_err("failed to deserialize CDR payload")
}

#[cfg(test)]
fn topic_to_zenoh_key(topic_name: &str) -> String {
    format!("rt/{}", topic_name.trim_start_matches('/'))
}

async fn robot_mode_worker(
    keep_running: CancellationToken,
    high_level_interface_client: Arc<BoosterClient>,
    robot_mode: Arc<Mutex<RobotMode>>,
) -> Result<()> {
    keep_running
        .run_until_cancelled(async {
            loop {
                match high_level_interface_client.get_mode().await {
                    Ok(get_mode_response) => {
                        let Some(received_robot_mode) = get_mode_response.mode_enum() else {
                            warn!("unrecognized robot mode id: {}", get_mode_response.mode);
                            continue;
                        };

                        *robot_mode.lock() = received_robot_mode;
                    }
                    Err(err) => {
                        error!("failed to get robot mode: {err}")
                    }
                }
            }
        })
        .await
        .ok_or(eyre!("termination_requested"))?;

    Ok(())
}

impl LowStateInterface for BoosterHardwareInterface {
    fn read_low_state(&self) -> Result<LowState> {
        let message = self
            .run_until_cancelled(self.low_state_receiver.lock().recv_latest())?
            .wrap_err("failed to read low state from `rt/low_state`")?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale low state messages from `rt/low_state`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }
}

impl LowCommandInterface for BoosterHardwareInterface {
    fn write_low_command(&self, low_command: LowCommand) -> Result<()> {
        self.run_until_cancelled(self.joint_control_sender.send(low_command))?
            .map_err(|_| eyre!("Zenoh publisher worker for `rt/joint_ctrl` closed"))?;
        Ok(())
    }
}

impl VisualKickInterface for BoosterHardwareInterface {
    fn write_visual_kick(&self, kick: Kick) -> Result<()> {
        self.run_until_cancelled(self.kick_ball_sender.send(kick))?
            .map_err(|_| eyre!("Zenoh publisher worker for `rt/kick_ball` closed"))?;
        Ok(())
    }
}

impl OdometerInterface for BoosterHardwareInterface {
    fn get_odometer(&self) -> Result<Odometer> {
        let message = self
            .run_until_cancelled(self.odometer_receiver.lock().recv_latest())?
            .wrap_err("failed to read odometer from `rt/odometer_state`")?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale odometer messages from `rt/odometer_state`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }
}

impl FallDownStateInterface for BoosterHardwareInterface {
    fn read_fall_down_state(&self) -> Result<FallDownState> {
        let message = self
            .run_until_cancelled(self.fall_down_state_receiver.lock().recv_latest())?
            .wrap_err("failed to read fall down state from `rt/fall_down`")?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale fall down state messages from `rt/fall_down`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }
}

impl ButtonEventMsgInterface for BoosterHardwareInterface {
    fn read_button_event_msg(&self) -> Result<ButtonEventMsg> {
        let message = self
            .run_until_cancelled(self.button_event_msg_receiver.lock().recv_latest())?
            .wrap_err("failed to read button event from `rt/button_event`")?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale button event messages from `rt/button_event`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }
}

impl RemoteControllerStateInterface for BoosterHardwareInterface {
    fn read_remote_controller_state(&self) -> Result<RemoteControllerState> {
        let message = self
            .run_until_cancelled(self.remote_controller_state_receiver.lock().recv_latest())?
            .wrap_err("failed to read remote controller state from `rt/remote_controller_state`")?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale remote controller state messages from `rt/remote_controller_state`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }
}

impl CameraInterface for BoosterHardwareInterface {
    fn read_rectified_image(&self) -> Result<Image> {
        let message = self
            .run_until_cancelled(self.rectified_image_receiver.lock().recv_latest())?
            .wrap_err("failed to read rectified image from `rt/StereoNetNode/rectified_image`")?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale rectified image messages from `rt/StereoNetNode/rectified_image`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }

    fn read_stereonet_depth_image(&self) -> Result<Image> {
        let message = self
            .run_until_cancelled(self.stereonet_depth_receiver.lock().recv_latest())?
            .wrap_err(
                "failed to read stereonet depth image from `rt/StereoNetNode/stereonet_depth`",
            )?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale stereonet depth image messages from `rt/StereoNetNode/stereonet_depth`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }

    fn read_stereonet_depth_camera_info(&self) -> Result<CameraInfo> {
        let message = self
            .run_until_cancelled(self.stereonet_depth_camera_info_receiver.lock().recv_latest())?
            .wrap_err("failed to read stereonet depth camera info from `rt/StereoNetNode/stereonet_depth/camera_info`")?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale stereonet depth camera info messages from `rt/StereoNetNode/stereonet_depth/camera_info`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }

    fn read_image_left_raw(&self) -> Result<Image> {
        let message = self
            .run_until_cancelled(self.image_left_raw_receiver.lock().recv_latest())?
            .wrap_err("failed to read left raw image from `rt/image_left_raw`")?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale left raw image messages from `rt/image_left_raw`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }

    fn read_image_left_raw_camera_info(&self) -> Result<CameraInfo> {
        let message = self
            .run_until_cancelled(
                self.image_left_raw_camera_info_receiver
                    .lock()
                    .recv_latest(),
            )?
            .wrap_err("failed to read left raw camera info from `rt/image_left_raw/camera_info`")?;
        if message.dropped_messages > 0 {
            debug!(
                "dropped {} stale left raw camera info messages from `rt/image_left_raw/camera_info`",
                message.dropped_messages
            );
        }
        Ok(message.value)
    }
}

impl TimeInterface for BoosterHardwareInterface {
    fn get_now(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl PathsInterface for BoosterHardwareInterface {
    fn get_paths(&self) -> Paths {
        self.paths.clone()
    }
}

impl NetworkInterface for BoosterHardwareInterface {
    fn read_from_network(&self) -> Result<IncomingMessage> {
        self.run_until_cancelled(self.hsl_network_endpoint.read())?
            .wrap_err("failed to read from network")
    }

    fn write_to_network(&self, message: OutgoingMessage) -> Result<()> {
        self.run_until_cancelled(self.hsl_network_endpoint.write(message))
    }
}

impl SpeakerInterface for BoosterHardwareInterface {
    fn write_to_speakers(&self, _request: SpeakerRequest) {
        log::debug!("tried to play audio request, not implemented!")
    }
}

impl IdInterface for BoosterHardwareInterface {
    fn get_ids(&self) -> Ids {
        self.ids.clone()
    }
}

impl MicrophoneInterface for BoosterHardwareInterface {
    fn read_from_microphones(&self) -> Result<Samples> {
        bail!("microphone interface is not implemented")
    }
}

impl RecordingInterface for BoosterHardwareInterface {
    fn should_record(&self) -> bool {
        self.enable_recording.load(Ordering::SeqCst)
    }

    fn set_whether_to_record(&self, enable: bool) {
        self.enable_recording.store(enable, Ordering::SeqCst)
    }
}

impl SimulatorInterface for BoosterHardwareInterface {
    fn is_simulation(&self) -> Result<bool> {
        Ok(false)
    }
}

impl HighLevelInterface for BoosterHardwareInterface {
    fn change_mode(&self, mode: RobotMode) -> Result<()> {
        self.run_until_cancelled(self.high_level_interface_client.change_mode(mode))?
            .wrap_err("failed to send change mode command")
    }

    fn get_mode(&self) -> Result<RobotMode> {
        Ok(*self.robot_mode.lock())
    }

    fn move_robot(&self, step: Step) -> Result<()> {
        self.run_until_cancelled(self.high_level_interface_client.move_robot(
            step.forward,
            step.left,
            step.turn,
        ))?
        .wrap_err("failed to send move robot command")
    }

    fn rotate_head(&self, head_joints: HeadJoints<f32>) -> Result<()> {
        self.run_until_cancelled(
            self.high_level_interface_client
                .rotate_head(head_joints.pitch, head_joints.yaw),
        )?
        .wrap_err("failed to send rotate head command")
    }

    fn rotate_head_with_direction(&self, head_joints: HeadJoints<i32>) -> Result<()> {
        self.run_until_cancelled(
            self.high_level_interface_client
                .rotate_head_with_direction(head_joints.pitch, head_joints.yaw),
        )?
        .wrap_err("failed to send rotate head with direction command")
    }

    fn lie_down(&self) -> Result<()> {
        self.run_until_cancelled(self.high_level_interface_client.lie_down())?
            .wrap_err("failed to send lie down command")
    }

    fn get_up(&self) -> Result<()> {
        self.run_until_cancelled(self.high_level_interface_client.get_up())?
            .wrap_err("failed to send get up command")
    }

    fn get_up_with_mode(&self, mode: RobotMode) -> Result<()> {
        self.run_until_cancelled(self.high_level_interface_client.get_up_with_mode(mode))?
            .wrap_err("failed to send get up with mode command")
    }

    fn enter_wbc_gait(&self) -> Result<()> {
        self.run_until_cancelled(self.high_level_interface_client.enter_wbc_gait())?
            .wrap_err("failed to send enter wbc gait command")
    }

    fn exit_wbc_gait(&self) -> Result<()> {
        self.run_until_cancelled(self.high_level_interface_client.exit_wbc_gait())?
            .wrap_err("failed to send exit wbc gait command")
    }

    fn visual_kick(&self, start: bool) -> Result<()> {
        self.run_until_cancelled(self.high_level_interface_client.visual_kick(start))?
            .wrap_err("failed to send visual kick command")
    }
}

impl MotionRuntimeInteface for BoosterHardwareInterface {
    fn get_motion_runtime_type(&self) -> Result<MotionRuntime> {
        Ok(MotionRuntime::Booster)
    }
}

impl HardwareInterface for BoosterHardwareInterface {}

#[cfg(test)]
mod tests {
    use booster::{CommandType, ImuState, LowCommand, LowState, MotorCommand};

    use super::{deserialize_zenoh_message, serialize_zenoh_message, topic_to_zenoh_key};

    #[test]
    fn topic_mapping_strips_leading_slashes() {
        assert_eq!(topic_to_zenoh_key("/image_left_raw"), "rt/image_left_raw");
        assert_eq!(
            topic_to_zenoh_key("/StereoNetNode/stereonet_depth/camera_info"),
            "rt/StereoNetNode/stereonet_depth/camera_info"
        );
        assert_eq!(topic_to_zenoh_key("low_state"), "rt/low_state");
    }

    #[test]
    fn low_state_cdr_round_trip_is_stable() {
        let message = LowState {
            imu_state: ImuState::default(),
            motor_state_parallel: Vec::new(),
            motor_state_serial: Vec::new(),
        };

        let encoded = serialize_zenoh_message(&message).unwrap();
        let decoded: LowState = deserialize_zenoh_message(&encoded).unwrap();
        let reencoded = serialize_zenoh_message(&decoded).unwrap();

        assert_eq!(decoded.imu_state, message.imu_state);
        assert_eq!(
            decoded.motor_state_parallel.len(),
            message.motor_state_parallel.len()
        );
        assert_eq!(
            decoded.motor_state_serial.len(),
            message.motor_state_serial.len()
        );
        assert_eq!(reencoded, encoded);
    }

    #[test]
    fn low_command_cdr_round_trip_is_stable() {
        let message = LowCommand {
            command_type: CommandType::Serial,
            motor_commands: vec![MotorCommand {
                command_type: CommandType::Serial,
                position: 1.0,
                velocity: 2.0,
                torque: 3.0,
                kp: 4.0,
                kd: 5.0,
                weight: 0.5,
            }],
        };

        let encoded = serialize_zenoh_message(&message).unwrap();
        let decoded: LowCommand = deserialize_zenoh_message(&encoded).unwrap();
        let reencoded = serialize_zenoh_message(&decoded).unwrap();

        assert_eq!(decoded.motor_commands.len(), 1);
        assert_eq!(decoded.motor_commands[0].position, 1.0);
        assert_eq!(decoded.motor_commands[0].velocity, 2.0);
        assert_eq!(decoded.motor_commands[0].torque, 3.0);
        assert_eq!(decoded.motor_commands[0].kp, 4.0);
        assert_eq!(decoded.motor_commands[0].kd, 5.0);
        assert_eq!(decoded.motor_commands[0].weight, 0.5);
        assert_eq!(reencoded, encoded);
    }
}
