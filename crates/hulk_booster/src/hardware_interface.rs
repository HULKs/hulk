use std::{
    env,
    future::{Future, IntoFuture},
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};

use booster_sdk::{
    client::BoosterClient,
    types::{GetModeResponse, RobotMode},
};
use cdr::{CdrLe, Infinite};
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail, eyre},
};
use kinematics::joints::head::HeadJoints;
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;
use zenoh::{
    Session,
    bytes::ZBytes,
    handlers::{RingChannel, RingChannelHandler},
    pubsub::{Publisher, Subscriber},
    sample::Sample,
};

use booster::{ButtonEventMsg, FallDownState, LowCommand, LowState, RemoteControllerState};
use hardware::{
    ButtonEventMsgInterface, CameraInterface, FallDownStateInterface, HighLevelInterface,
    IdInterface, LowCommandInterface, LowStateInterface, MicrophoneInterface, NetworkInterface,
    PathsInterface, RecordingInterface, RemoteControllerStateInterface, SimulatorInterface,
    SpeakerInterface, TimeInterface,
};
use hsl_network::endpoint::{Endpoint, Ports};
use hula_types::hardware::{Ids, Paths};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use types::{
    audio::SpeakerRequest,
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    step::Step,
};

use crate::HardwareInterface;

struct TopicInfos {
    low_state: TopicInfo,
    joint_ctrl: TopicInfo,
    fall_down: TopicInfo,
    button_event: TopicInfo,
    remote_controller_state: TopicInfo,
    rectified_image: TopicInfo,
    stereonet_depth: TopicInfo,
    stereonet_depth_camera_info: TopicInfo,
    image_left_raw: TopicInfo,
    image_left_raw_camera_info: TopicInfo,
}

impl Default for TopicInfos {
    fn default() -> Self {
        Self {
            low_state: TopicInfo::new("booster/low_state"),
            joint_ctrl: TopicInfo::new("booster/joint_ctrl"),
            fall_down: TopicInfo::new("booster/fall_down_state"),
            button_event: TopicInfo::new("booster/button_event"),
            remote_controller_state: TopicInfo::new("booster/remote_controller_state"),
            rectified_image: TopicInfo::new("StereoNetNode/rectified_image"),
            stereonet_depth: TopicInfo::new("StereoNetNode/stereonet_depth"),
            stereonet_depth_camera_info: TopicInfo::new(
                "StereoNetNode/stereonet_depth/camera_info",
            ),
            image_left_raw: TopicInfo::new("image_left_raw"),
            image_left_raw_camera_info: TopicInfo::new("image_left_raw/camera_info"),
        }
    }
}

struct TopicInfo {
    pub name: &'static str,
}

impl TopicInfo {
    const fn new(name: &'static str) -> Self {
        TopicInfo { name }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub dds_domain_id: u16,
    pub hsl_network_ports: Ports,
    pub paths: Paths,
}

pub struct BoosterHardwareInterface {
    ids: Ids,
    paths: Paths,
    enable_recording: AtomicBool,

    low_state_subscriber: Subscriber<RingChannelHandler<Sample>>,
    joint_control_publisher: Publisher<'static>,
    fall_down_state_subscriber: Subscriber<RingChannelHandler<Sample>>,
    button_event_msg_subscriber: Subscriber<RingChannelHandler<Sample>>,
    remote_controller_state_subscriber: Subscriber<RingChannelHandler<Sample>>,
    rectified_image_subscriber: Subscriber<RingChannelHandler<Sample>>,
    stereonet_depth_subscriber: Subscriber<RingChannelHandler<Sample>>,
    stereonet_depth_camera_info_subscriber: Subscriber<RingChannelHandler<Sample>>,
    image_left_raw_subscriber: Subscriber<RingChannelHandler<Sample>>,
    image_left_raw_camera_info_subscriber: Subscriber<RingChannelHandler<Sample>>,

    high_level_interface_client: BoosterClient,

    _session: Session,
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
        let session = zenoh::open(zenoh::Config::default())
            .await
            .expect("failed to open zenoh session");

        let topic_infos = TopicInfos::default();

        let Some(hardware_id) = env::var_os("HARDWARE_ID") else {
            bail!("environment variable HARDWARE_ID not set")
        };
        let ids = Ids {
            robot_id: hardware_id
                .into_string()
                .ok()
                .wrap_err("id was not valid UTF-8")?,
        };

        let high_level_interface_client = BoosterClient::new()?;

        Ok(Self {
            ids,
            paths: parameters.paths,
            enable_recording: AtomicBool::new(false),

            low_state_subscriber: declare_subscriber(&session, &topic_infos.low_state).await?,
            joint_control_publisher: declare_publisher(&session, &topic_infos.joint_ctrl).await?,
            fall_down_state_subscriber: declare_subscriber(&session, &topic_infos.fall_down)
                .await?,
            button_event_msg_subscriber: declare_subscriber(&session, &topic_infos.button_event)
                .await?,
            remote_controller_state_subscriber: declare_subscriber(
                &session,
                &topic_infos.remote_controller_state,
            )
            .await?,
            rectified_image_subscriber: declare_subscriber(&session, &topic_infos.rectified_image)
                .await?,
            stereonet_depth_subscriber: declare_subscriber(&session, &topic_infos.stereonet_depth)
                .await?,
            stereonet_depth_camera_info_subscriber: declare_subscriber(
                &session,
                &topic_infos.stereonet_depth_camera_info,
            )
            .await?,
            image_left_raw_subscriber: declare_subscriber(&session, &topic_infos.image_left_raw)
                .await?,
            image_left_raw_camera_info_subscriber: declare_subscriber(
                &session,
                &topic_infos.image_left_raw_camera_info,
            )
            .await?,

            high_level_interface_client,
            _session: session,
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

async fn declare_subscriber(
    session: &Session,
    topic_info: &TopicInfo,
) -> Result<Subscriber<RingChannelHandler<Sample>>> {
    session
        .declare_subscriber(topic_info.name)
        .with(RingChannel::new(10))
        .await
        .map_err(|err| eyre!(err))
}

async fn declare_publisher(
    session: &Session,
    topic_info: &TopicInfo,
) -> Result<Publisher<'static>> {
    let key_expression = session
        .declare_keyexpr(topic_info.name)
        .await
        .map_err(|err| eyre!(err))?;

    session
        .declare_publisher(key_expression)
        .await
        .map_err(|err| eyre!(err))
}

fn deserialize_sample<T>(sample: Sample) -> Result<T>
where
    for<'de> T: Deserialize<'de>,
{
    let deserialized_message = cdr::deserialize(&sample.payload().to_bytes())?;
    Ok(deserialized_message)
}

fn serialize_sample<T>(payload: T) -> Result<ZBytes>
where
    T: Serialize,
{
    let message = cdr::serialize::<_, _, CdrLe>(&payload, Infinite)?;

    Ok(ZBytes::from(message))
}

impl LowStateInterface for BoosterHardwareInterface {
    fn read_low_state(&self) -> Result<LowState> {
        self.run_until_cancelled(self.low_state_subscriber.recv_async())?
            .map_err(|error| eyre!(error))
            .and_then(deserialize_sample)
    }
}

impl LowCommandInterface for BoosterHardwareInterface {
    fn write_low_command(&self, low_command: LowCommand) -> Result<()> {
        let payload = serialize_sample(low_command)?;

        self.run_until_cancelled(self.joint_control_publisher.put(payload).into_future())?
            .map_err(|error| eyre!(error))
    }
}

impl FallDownStateInterface for BoosterHardwareInterface {
    fn read_fall_down_state(&self) -> Result<FallDownState> {
        self.run_until_cancelled(self.fall_down_state_subscriber.recv_async())?
            .map_err(|error| eyre!(error))
            .and_then(deserialize_sample)
    }
}

impl ButtonEventMsgInterface for BoosterHardwareInterface {
    fn read_button_event_msg(&self) -> Result<ButtonEventMsg> {
        self.run_until_cancelled(self.button_event_msg_subscriber.recv_async())?
            .map_err(|error| eyre!(error))
            .and_then(deserialize_sample)
    }
}

impl RemoteControllerStateInterface for BoosterHardwareInterface {
    fn read_remote_controller_state(&self) -> Result<RemoteControllerState> {
        self.run_until_cancelled(self.remote_controller_state_subscriber.recv_async())?
            .map_err(|error| eyre!(error))
            .and_then(deserialize_sample)
    }
}

impl CameraInterface for BoosterHardwareInterface {
    fn read_rectified_image(&self) -> Result<Image> {
        self.run_until_cancelled(self.rectified_image_subscriber.recv_async())?
            .map_err(|error| eyre!(error))
            .and_then(deserialize_sample)
    }

    fn read_stereonet_depth_image(&self) -> Result<Image> {
        self.run_until_cancelled(self.stereonet_depth_subscriber.recv_async())?
            .map_err(|error| eyre!(error))
            .and_then(deserialize_sample)
    }

    fn read_stereonet_depth_camera_info(&self) -> Result<CameraInfo> {
        self.run_until_cancelled(self.stereonet_depth_camera_info_subscriber.recv_async())?
            .map_err(|error| eyre!(error))
            .and_then(deserialize_sample)
    }

    fn read_image_left_raw(&self) -> Result<Image> {
        self.run_until_cancelled(self.image_left_raw_subscriber.recv_async())?
            .map_err(|error| eyre!(error))
            .and_then(deserialize_sample)
    }

    fn read_image_left_raw_camera_info(&self) -> Result<CameraInfo> {
        self.run_until_cancelled(self.image_left_raw_camera_info_subscriber.recv_async())?
            .map_err(|error| eyre!(error))
            .and_then(deserialize_sample)
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
        log::warn!("tried to play audio request, not implemented!")
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

    fn get_mode(&self) -> Result<GetModeResponse> {
        self.run_until_cancelled(self.high_level_interface_client.get_mode())?
            .wrap_err("failed to send get mode request")
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
}

impl HardwareInterface for BoosterHardwareInterface {}
