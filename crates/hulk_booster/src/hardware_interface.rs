use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use booster::{ButtonEventMsg, FallDownState, LowCommand, LowState, RemoteControllerState};
use color_eyre::eyre::{eyre, Context, ContextCompat};
use color_eyre::Result;
use hardware::{
    ButtonEventMsgInterface, CameraInterface, IdInterface, MicrophoneInterface, NetworkInterface,
    PathsInterface, RecordingInterface, SpeakerInterface, TimeInterface,
};
use hardware::{
    FallDownStateInterface, LowCommandInterface, LowStateInterface, RemoteControllerStateInterface,
};
use hula_types::hardware::{Ids, Paths};
use parking_lot::Mutex;
use rustdds::no_key::DataReader;
use rustdds::no_key::DataWriter;
use rustdds::DomainParticipant;
use rustdds::Publisher;
use rustdds::QosPolicies;
use rustdds::Subscriber;
use rustdds::Topic;
use serde::Deserialize;
use types::audio::SpeakerRequest;
use types::messages::{IncomingMessage, OutgoingMessage};
use types::samples::Samples;

use crate::HardwareInterface;

const FIND_TOPIC_TIMEOUT: Duration = Duration::from_secs(1);

struct TopicInfos {
    low_state: TopicInfo,
    joint_ctrl: TopicInfo,
    fall_down: TopicInfo,
    button_event: TopicInfo,
    remote_controller_state: TopicInfo,
    // transform: TopicInfo,
}

impl Default for TopicInfos {
    fn default() -> Self {
        Self {
            low_state: TopicInfo::new(
                "rt/low_state",
                "Obtain the robot's IMU and joint feedback in real time.",
            ),
            joint_ctrl: TopicInfo::new(
                "rt/joint_ctrl",
                "Publish the joint commands of the robot to control the motors.",
            ),
            fall_down: TopicInfo::new("rt/fall_down", "Real-time detection of robot falls"),
            button_event: TopicInfo::new(
                "rt/button_event",
                "Real-time retrieval of backboard button inputs",
            ),
            remote_controller_state: TopicInfo::new(
                "rt/remote_controller_state",
                "Real-time retrieval of remote controller button inputs",
            ),
            // transform: TopicInfo::new(
            //     "rt/tf",
            //     "Real-time acquisition of coordinate transformations between robot joints",
            // ),
        }
    }
}

struct TopicInfo {
    pub name: &'static str,
    pub description: &'static str,
}

impl TopicInfo {
    const fn new(name: &'static str, description: &'static str) -> Self {
        TopicInfo { name, description }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub dds_domain_id: u16,
    pub paths: Paths,
}

pub struct BoosterHardwareInterface {
    paths: Paths,
    enable_recording: AtomicBool,
    time: Arc<Mutex<SystemTime>>,

    low_state_reader: Mutex<DataReader<LowState>>,
    joint_control_writer: Mutex<DataWriter<LowCommand>>,
    fall_down_state_reader: Mutex<DataReader<FallDownState>>,
    button_event_msg_reader: Mutex<DataReader<ButtonEventMsg>>,
    remote_controller_state_reader: Mutex<DataReader<RemoteControllerState>>,
    _participant: DomainParticipant,
    _subscriber: Subscriber,
    _publisher: Publisher,
}

impl BoosterHardwareInterface {
    pub fn new(parameters: Parameters) -> Result<Self> {
        let participant = DomainParticipant::new(parameters.dds_domain_id)?;
        let subscriber = participant.create_subscriber(&QosPolicies::qos_none())?;
        let publisher = participant.create_publisher(&QosPolicies::qos_none())?;

        let topic_infos = TopicInfos::default();

        let low_state_reader = Mutex::new(subscriber.create_datareader_no_key_cdr(
            &find_or_create_topic(&participant, topic_infos.low_state),
            None,
        )?);
        let joint_control_writer = Mutex::new(publisher.create_datawriter_no_key_cdr(
            &find_or_create_topic(&participant, topic_infos.joint_ctrl),
            None,
        )?);

        let fall_down_state_reader = Mutex::new(subscriber.create_datareader_no_key_cdr(
            &find_or_create_topic(&participant, topic_infos.fall_down),
            None,
        )?);
        let button_event_msg_reader = Mutex::new(subscriber.create_datareader_no_key_cdr(
            &find_or_create_topic(&participant, topic_infos.button_event),
            None,
        )?);
        let remote_controller_state_reader = Mutex::new(subscriber.create_datareader_no_key_cdr(
            &find_or_create_topic(&participant, topic_infos.remote_controller_state),
            None,
        )?);
        // let transform_stamped_reader = Mutex::new(subscriber.create_datareader_no_key_cdr(
        //     &find_or_create_topic(&participant, topic_infos.transform),
        //     None,
        // )?);
        //

        let time = Arc::new(Mutex::new(SystemTime::UNIX_EPOCH));

        Ok(Self {
            paths: parameters.paths,
            enable_recording: AtomicBool::new(false),
            time,

            low_state_reader,
            joint_control_writer,
            fall_down_state_reader,
            button_event_msg_reader,
            remote_controller_state_reader,

            // TODO: Necessary?
            _participant: participant,
            _subscriber: subscriber,
            _publisher: publisher,
        })
    }
}

fn find_or_create_topic(participant: &DomainParticipant, topic_info: TopicInfo) -> Topic {
    if let Some(topic) = participant
        .find_topic(topic_info.name, FIND_TOPIC_TIMEOUT)
        .ok()
        .flatten()
    {
        topic
    } else {
        dbg!("Creating topic..");
        participant
            .create_topic(
                "EEEEEELSE".to_string(),
                topic_info.description.to_string(),
                &QosPolicies::qos_none(),
                rustdds::TopicKind::NoKey,
            )
            .unwrap()
    }
}

impl LowStateInterface for BoosterHardwareInterface {
    fn read_low_state(&self) -> Result<LowState> {
        let sample = self.low_state_reader.lock().take_next_sample()?;

        Ok(sample.wrap_err("no data")?.into_value())
    }
}

impl LowCommandInterface for BoosterHardwareInterface {
    fn write_low_command(&self, low_command: LowCommand) -> Result<()> {
        self.joint_control_writer
            .lock()
            .write(low_command, None)
            .wrap_err("failed to write joint control")
    }
}

impl FallDownStateInterface for BoosterHardwareInterface {
    fn read_fall_down_state(&self) -> Result<FallDownState> {
        let sample = self.fall_down_state_reader.lock().take_next_sample()?;

        Ok(sample.wrap_err("no data")?.into_value())
    }
}

impl ButtonEventMsgInterface for BoosterHardwareInterface {
    fn read_button_event_msg(&self) -> Result<ButtonEventMsg> {
        let sample = self.button_event_msg_reader.lock().take_next_sample()?;

        Ok(sample.wrap_err("no data")?.into_value())
    }
}

impl RemoteControllerStateInterface for BoosterHardwareInterface {
    fn read_remote_controller_state(&self) -> Result<RemoteControllerState> {
        let sample = self
            .remote_controller_state_reader
            .lock()
            .take_next_sample()?;

        Ok(sample.wrap_err("no data")?.into_value())
    }
}

impl CameraInterface for BoosterHardwareInterface {
    fn read_image(&self) -> Result<ros2::sensor_msgs::image::Image> {
        todo!()
    }

    fn read_camera_info(&self) -> Result<ros2::sensor_msgs::camera_info::CameraInfo> {
        todo!()
    }
}

impl TimeInterface for BoosterHardwareInterface {
    fn get_now(&self) -> SystemTime {
        *self.time.lock()
    }
}

impl PathsInterface for BoosterHardwareInterface {
    fn get_paths(&self) -> Paths {
        self.paths.clone()
    }
}

impl NetworkInterface for BoosterHardwareInterface {
    fn read_from_network(&self) -> Result<IncomingMessage> {
        todo!()
    }

    fn write_to_network(&self, _message: OutgoingMessage) -> Result<()> {
        todo!()
    }
}

impl SpeakerInterface for BoosterHardwareInterface {
    fn write_to_speakers(&self, _request: SpeakerRequest) {
        todo!()
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
