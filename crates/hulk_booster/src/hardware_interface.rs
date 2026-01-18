use std::future::IntoFuture;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use booster::{
    ButtonEventMsg, FallDownState, LowCommand, LowState, RemoteControllerState, TransformMessage,
};
use byteorder::{BigEndian, LittleEndian};
use bytes::Bytes;
use color_eyre::eyre::{eyre, Context, Error};
use color_eyre::Result;
use hardware::{
    ButtonEventMsgInterface, CameraInterface, IdInterface, MicrophoneInterface, NetworkInterface,
    PathsInterface, RecordingInterface, SpeakerInterface, TimeInterface, TransformMessageInterface,
};
use hardware::{
    FallDownStateInterface, LowCommandInterface, LowStateInterface, RemoteControllerStateInterface,
};
use hsl_network::endpoint::{Endpoint, Ports};
use hula_types::hardware::{Ids, Paths};
use ros2::sensor_msgs::camera_info::CameraInfo;
use ros2::sensor_msgs::image::Image;
use rustdds::CdrDeserializer;
use serde::Deserialize;
use tokio::runtime::Handle;
use tokio::select;
use tokio_util::sync::CancellationToken;
use types::audio::SpeakerRequest;
use types::messages::{IncomingMessage, OutgoingMessage};
use types::samples::Samples;
use zenoh::bytes::ZBytes;
use zenoh::handlers::{RingChannel, RingChannelHandler};
use zenoh::pubsub::{Publisher, Subscriber};
use zenoh::sample::Sample;
use zenoh::Session;

use crate::HardwareInterface;

#[derive(Deserialize)]
struct DDSDataWrapper {
    representation_identifier: [u8; 2],
    representation_options: [u8; 2],
    bytes: Bytes,
}

impl DDSDataWrapper {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            representation_identifier: bytes[0..2].try_into().unwrap(),
            representation_options: bytes[2..4].try_into().unwrap(),
            bytes: bytes[4..].to_owned().into(),
        }
    }
}

struct TopicInfos {
    low_state: TopicInfo,
    joint_ctrl: TopicInfo,
    fall_down: TopicInfo,
    button_event: TopicInfo,
    remote_controller_state: TopicInfo,
    transform: TopicInfo,
    rectified_image: TopicInfo,
    image_left_raw: TopicInfo,
    image_left_raw_camera_info: TopicInfo,
}

impl Default for TopicInfos {
    fn default() -> Self {
        Self {
            low_state: TopicInfo::new("booster/low_state"),
            joint_ctrl: TopicInfo::new("booster/joint_ctrl"),
            fall_down: TopicInfo::new("booster/fall_down"),
            button_event: TopicInfo::new("booster/button_event"),
            remote_controller_state: TopicInfo::new("booster/remote_controller_state"),
            transform: TopicInfo::new("booster/tf"),
            rectified_image: TopicInfo::new("booster/rectified_image"),
            image_left_raw: TopicInfo::new("booster/image_left_raw"),
            image_left_raw_camera_info: TopicInfo::new("booster/image_left_raw/camera_info"),
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
    paths: Paths,
    enable_recording: AtomicBool,

    low_state_subscriber: Subscriber<RingChannelHandler<Sample>>,
    joint_control_publisher: Publisher<'static>,
    fall_down_state_subscriber: Subscriber<RingChannelHandler<Sample>>,
    button_event_msg_subscriber: Subscriber<RingChannelHandler<Sample>>,
    remote_controller_state_subscriber: Subscriber<RingChannelHandler<Sample>>,
    transform_subscriber: Subscriber<RingChannelHandler<Sample>>,
    rectified_image_subscriber: Subscriber<RingChannelHandler<Sample>>,
    image_left_raw_subscriber: Subscriber<RingChannelHandler<Sample>>,
    image_left_raw_camera_info_subscriber: Subscriber<RingChannelHandler<Sample>>,

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
        let session = zenoh::open(zenoh::Config::default()).await.unwrap();

        let topic_infos = TopicInfos::default();

        Ok(Self {
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
            transform_subscriber: declare_subscriber(&session, &topic_infos.transform).await?,
            rectified_image_subscriber: declare_subscriber(&session, &topic_infos.rectified_image)
                .await?,
            image_left_raw_subscriber: declare_subscriber(&session, &topic_infos.image_left_raw)
                .await?,
            image_left_raw_camera_info_subscriber: declare_subscriber(
                &session,
                &topic_infos.image_left_raw_camera_info,
            )
            .await?,

            _session: session,
            hsl_network_endpoint: tokio::task::block_in_place(|| {
                runtime_handle.block_on(Endpoint::new(parameters.hsl_network_ports))
            })
            .wrap_err("failed to initialize HSL network")?,
            runtime_handle,
            keep_running,
        })
    }
}

async fn declare_subscriber(
    session: &Session,
    topic_info: &TopicInfo,
) -> Result<Subscriber<RingChannelHandler<Sample>>> {
    let key_expression = session
        .declare_keyexpr(topic_info.name)
        .await
        .map_err(|err| eyre!(err))?;

    session
        .declare_subscriber(key_expression)
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
    let ddsdata_wrapper = DDSDataWrapper::from_bytes(&sample.payload().to_bytes());
    match ddsdata_wrapper.representation_identifier {
        [0x00, 0x01] => {
            let mut deserializer = CdrDeserializer::<LittleEndian>::new(&ddsdata_wrapper.bytes);
            serde::de::Deserialize::deserialize(&mut deserializer).map_err(|err| eyre!(err))
        }
        [0x00, 0x00] => {
            let mut deserializer = CdrDeserializer::<BigEndian>::new(&ddsdata_wrapper.bytes);
            serde::de::Deserialize::deserialize(&mut deserializer).map_err(|err| eyre!(err))
        }
        _ => Err(eyre!(
            "Representation identifier {:#?} not supported",
            ddsdata_wrapper.representation_identifier
        )),
    }
}

fn serialize_sample<T>(payload: T) -> ZBytes {
    todo!();
}

impl LowStateInterface for BoosterHardwareInterface {
    fn read_low_state(&self) -> Result<LowState> {
        let sample = self.low_state_subscriber.recv().map_err(|e| eyre!("{e}"))?;
        deserialize_sample(sample)
    }
}

impl LowCommandInterface for BoosterHardwareInterface {
    fn write_low_command(&self, low_command: LowCommand) -> Result<()> {
        let payload = serialize_sample(low_command);

        self.runtime_handle
            .block_on(self.joint_control_publisher.put(payload).into_future())
            .map_err(|err| eyre!(err))
    }
}

impl FallDownStateInterface for BoosterHardwareInterface {
    fn read_fall_down_state(&self) -> Result<FallDownState> {
        let sample = self
            .fall_down_state_subscriber
            .recv()
            .map_err(|e| eyre!("{e}"))?;
        deserialize_sample(sample)
    }
}

impl ButtonEventMsgInterface for BoosterHardwareInterface {
    fn read_button_event_msg(&self) -> Result<ButtonEventMsg> {
        let sample = self
            .button_event_msg_subscriber
            .recv()
            .map_err(|e| eyre!("{e}"))?;
        deserialize_sample(sample)
    }
}

impl TransformMessageInterface for BoosterHardwareInterface {
    fn read_transform_message(&self) -> Result<TransformMessage> {
        let sample = self.transform_subscriber.recv().map_err(|e| eyre!("{e}"))?;
        deserialize_sample(sample)
    }
}

impl RemoteControllerStateInterface for BoosterHardwareInterface {
    fn read_remote_controller_state(&self) -> Result<RemoteControllerState> {
        let sample = self
            .remote_controller_state_subscriber
            .recv()
            .map_err(|e| eyre!("{e}"))?;
        deserialize_sample(sample)
    }
}

impl CameraInterface for BoosterHardwareInterface {
    fn read_image_left_raw(&self) -> Result<Image> {
        let sample = self
            .image_left_raw_subscriber
            .recv()
            .map_err(|e| eyre!("{e}"))?;
        deserialize_sample(sample)
    }

    fn read_image_left_raw_camera_info(&self) -> Result<CameraInfo> {
        let sample = self
            .image_left_raw_camera_info_subscriber
            .recv()
            .map_err(|e| eyre!("{e}"))?;
        deserialize_sample(sample)
    }

    fn read_stereonet_visual(&self) -> Result<Image> {
        todo!()
    }

    fn read_stereonet_camera_info(&self) -> Result<CameraInfo> {
        todo!()
    }

    fn read_rectified_image(&self) -> Result<Image> {
        let sample = self
            .rectified_image_subscriber
            .recv()
            .map_err(|e| eyre!("{e}"))?;
        deserialize_sample(sample)
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
        self.runtime_handle.block_on(async {
            select! {
                result = self.hsl_network_endpoint.read() => {
                    result.map_err(Error::from)
                },
                _ = self.keep_running.cancelled() => {
                    Err(eyre!("termination requested"))
                }
            }
        })
    }

    fn write_to_network(&self, message: OutgoingMessage) -> Result<()> {
        self.runtime_handle
            .block_on(self.hsl_network_endpoint.write(message));
        Ok(())
    }
}

impl SpeakerInterface for BoosterHardwareInterface {
    fn write_to_speakers(&self, _request: SpeakerRequest) {
        log::warn!("Tried to play audio request, not implemented!")
    }
}

impl IdInterface for BoosterHardwareInterface {
    fn get_ids(&self) -> Ids {
        let name = "Booster K1";
        Ids {
            body_id: name.to_string(),
            head_id: name.to_string(),
        }
    }
}

impl MicrophoneInterface for BoosterHardwareInterface {
    fn read_from_microphones(&self) -> Result<Samples> {
        Err(eyre!("microphone interface is not implemented"))
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

impl HardwareInterface for BoosterHardwareInterface {}
