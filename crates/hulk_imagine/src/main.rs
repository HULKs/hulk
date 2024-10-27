#![recursion_limit = "256"]
use std::collections::BTreeMap;
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Seek, Write};
use std::time::SystemTime;
use std::{env::args, path::PathBuf, sync::Arc};

use color_eyre::eyre::bail;
use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use hardware::{
    ActuatorInterface, CameraInterface, NetworkInterface, PathsInterface, RecordingInterface,
    SpeakerInterface, TimeInterface,
};
use indicatif::ProgressIterator;
use mcap::records::{system_time_to_nanos, MessageHeader};
use mcap::{Channel, McapError, Writer};
use serde::Serialize;
// use rmp_serde::to_vec;
use serde_json::{to_value, Value};

use types::hardware::Ids;
use types::{
    audio::SpeakerRequest,
    camera_position::CameraPosition,
    hardware::Paths,
    joints::Joints,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    ycbcr422_image::YCbCr422Image,
};

use crate::execution::Replayer;

pub trait HardwareInterface:
    ActuatorInterface
    + CameraInterface
    + NetworkInterface
    + PathsInterface
    + RecordingInterface
    + SpeakerInterface
    + TimeInterface
{
}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

struct ExtractorHardwareInterface;

impl ActuatorInterface for ExtractorHardwareInterface {
    fn write_to_actuators(
        &self,
        _positions: Joints<f32>,
        _stiffnesses: Joints<f32>,
        _leds: Leds,
    ) -> Result<()> {
        Ok(())
    }
}

impl CameraInterface for ExtractorHardwareInterface {
    fn read_from_camera(&self, _camera_position: CameraPosition) -> Result<YCbCr422Image> {
        panic!("Replayer cannot produce data from hardware")
    }
}

impl NetworkInterface for ExtractorHardwareInterface {
    fn read_from_network(&self) -> Result<IncomingMessage> {
        unimplemented!()
    }

    fn write_to_network(&self, _message: OutgoingMessage) -> Result<()> {
        Ok(())
    }
}
impl RecordingInterface for ExtractorHardwareInterface {
    fn should_record(&self) -> bool {
        false
    }

    fn set_whether_to_record(&self, _enable: bool) {}
}

impl SpeakerInterface for ExtractorHardwareInterface {
    fn write_to_speakers(&self, _request: SpeakerRequest) {}
}

impl PathsInterface for ExtractorHardwareInterface {
    fn get_paths(&self) -> Paths {
        Paths {
            motions: "etc/motions".into(),
            neural_networks: "etc/neural_networks".into(),
            sounds: "etc/sounds".into(),
        }
    }
}

impl TimeInterface for ExtractorHardwareInterface {
    fn get_now(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl HardwareInterface for ExtractorHardwareInterface {}

fn main() -> Result<()> {
    install()?;

    let replay_path = args()
        .nth(1)
        .expect("expected replay path as first parameter");

    let output_folder = PathBuf::from(
        args()
            .nth(2)
            .expect("expected output path as second parameter"),
    );

    let parameters_directory = args().nth(3).unwrap_or(replay_path.clone());
    let ids = Ids {
        body_id: "replayer".into(),
        head_id: "replayer".into(),
    };

    let mut replayer = Replayer::new(
        Arc::new(ExtractorHardwareInterface),
        parameters_directory,
        ids,
        replay_path,
    )
    .wrap_err("failed to create image extractor")?;

    let mut control_receiver = replayer.control_receiver();

    let output_folder = &output_folder.join("Control");
    create_dir_all(output_folder).expect("failed to create output folder");

    let output_file = output_folder.join("control_outputs.mcap");

    let mut mcap_converter =
        McapConverter::from_writer(BufWriter::new(File::create(output_file)?))?;

    let unknown_indices_error_message = format!("could not find recording indices for `Control`");
    let timings: Vec<_> = replayer
        .get_recording_indices()
        .get("Control")
        .expect(&unknown_indices_error_message)
        .iter()
        .collect();

    for (index, timing) in timings.iter().enumerate().progress() {
        let frame = replayer
            .get_recording_indices_mut()
            .get_mut("Control")
            .map(|index| {
                index
                    .find_latest_frame_up_to(timing.timestamp)
                    .expect("failed to find latest frame")
            })
            .expect(&unknown_indices_error_message);

        if let Some(frame) = frame {
            replayer
                .replay("Control", frame.timing.timestamp, &frame.data)
                .expect("failed to replay frame");

            let (_, database) = &*control_receiver.borrow_and_mark_as_seen();

            let values = database_to_values(database, "Control".to_string())?;
            values
                .into_iter()
                .map(|(topic, data)| {
                    mcap_converter.add_to_mcap(topic, data, index as u32, timing.timestamp)
                })
                .collect::<Result<_, _>>()?;
        }
    }

    // let buffer = to_string(&databases).expect("failed to serialize to MessagePack byte vector");

    // let output_file = output_folder.join("control_outputs.json");
    // write(output_file, buffer)?;

    mcap_converter.finish()?;

    Ok(())
}

type ChannelId = u16;
struct McapConverter<'a, W: Write + Seek> {
    writer: Writer<'a, W>,
    channel_mapping: BTreeMap<String, ChannelId>,
}

impl<'a, W: Write + Seek> McapConverter<'a, W> {
    pub fn from_writer(writer: W) -> Result<Self, McapError> {
        Ok(Self {
            writer: Writer::new(writer)?,
            channel_mapping: BTreeMap::default(),
        })
    }

    fn create_new_channel(&mut self, topic: String) -> Result<ChannelId, McapError> {
        let channel = Channel {
            topic: topic.clone(),
            schema: None,
            message_encoding: String::from("json"),
            metadata: BTreeMap::default(),
        };

        let channel_id = self.writer.add_channel(&channel)?;
        self.channel_mapping.insert(topic, channel_id);

        Ok(channel_id)
    }

    pub fn add_to_mcap(
        &mut self,
        topic: String,
        data: Value,
        sequence_number: u32,
        system_time: SystemTime,
    ) -> Result<(), McapError> {
        let channel_id = match self.channel_mapping.get(&topic).copied() {
            Some(channel_id) => channel_id,
            None => self.create_new_channel(topic)?,
        };
        let log_time = system_time_to_nanos(&system_time);

        self.writer.write_to_known_channel(
            &MessageHeader {
                channel_id,
                sequence: sequence_number,
                log_time: log_time,
                publish_time: log_time,
            },
            data.to_string().as_bytes(),
        )?;

        Ok(())
    }

    pub fn finish(mut self) -> Result<(), McapError> {
        self.writer.finish()
    }
}

pub fn database_to_values<T: Serialize>(
    database: T,
    cycler_name: String,
) -> Result<Vec<(String, Value)>> {
    let database_values = to_value(database).expect(&format!(
        "failed to serialize `{cycler_name}` database to serde json value"
    ));

    let mut map = Vec::new();

    let Value::Object(outputs) = database_values else {
        bail!("expected `Value::Object`")
    };

    for (output_type, cycler_output) in outputs {
        let Value::Object(values) = cycler_output else {
            bail!("expected `Value::Object`")
        };

        for (node_output_name, node_output) in values {
            let key = format!("{cycler_name}.{output_type}.{node_output_name}");
            map.push((key, node_output));
        }
    }

    Ok(map)
}
