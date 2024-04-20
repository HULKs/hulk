#![recursion_limit = "256"]
use std::time::SystemTime;
use std::{env::args, path::PathBuf, sync::Arc};

use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use hardware::{
    ActuatorInterface, CameraInterface, IdInterface, MicrophoneInterface, NetworkInterface,
    PathsInterface, RecordingInterface, SensorInterface, SpeakerInterface, TimeInterface,
};
use types::{
    audio::SpeakerRequest,
    camera_position::CameraPosition,
    hardware::{Ids, Paths},
    joints::Joints,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    sensor_data::SensorData,
    ycbcr422_image::YCbCr422Image,
};

use crate::execution::ImageExtractor;

pub trait HardwareInterface:
    ActuatorInterface
    + CameraInterface
    + IdInterface
    + MicrophoneInterface
    + NetworkInterface
    + PathsInterface
    + RecordingInterface
    + SensorInterface
    + SpeakerInterface
    + TimeInterface
{
}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

struct ReplayerHardwareInterface {
    ids: Ids,
}

impl ActuatorInterface for ReplayerHardwareInterface {
    fn write_to_actuators(
        &self,
        _positions: Joints<f32>,
        _stiffnesses: Joints<f32>,
        _leds: Leds,
    ) -> Result<()> {
        Ok(())
    }
}

impl CameraInterface for ReplayerHardwareInterface {
    fn read_from_camera(&self, _camera_position: CameraPosition) -> Result<YCbCr422Image> {
        panic!("Replayer cannot produce data from hardware")
    }
}

impl IdInterface for ReplayerHardwareInterface {
    fn get_ids(&self) -> Ids {
        self.ids.clone()
    }
}

impl MicrophoneInterface for ReplayerHardwareInterface {
    fn read_from_microphones(&self) -> Result<Samples> {
        panic!("Replayer cannot produce data from hardware")
    }
}

impl NetworkInterface for ReplayerHardwareInterface {
    fn read_from_network(&self) -> Result<IncomingMessage> {
        panic!("Replayer cannot produce data from hardware")
    }

    fn write_to_network(&self, _message: OutgoingMessage) -> Result<()> {
        Ok(())
    }
}

impl PathsInterface for ReplayerHardwareInterface {
    fn get_paths(&self) -> Paths {
        Paths {
            motions: "etc/motions".into(),
            neural_networks: "etc/neural_networks".into(),
            sounds: "etc/sounds".into(),
        }
    }
}

impl RecordingInterface for ReplayerHardwareInterface {
    fn should_record(&self) -> bool {
        false
    }

    fn set_whether_to_record(&self, _enable: bool) {}
}

impl SensorInterface for ReplayerHardwareInterface {
    fn read_from_sensors(&self) -> Result<SensorData> {
        panic!("Replayer cannot produce data from hardware")
    }
}

impl SpeakerInterface for ReplayerHardwareInterface {
    fn write_to_speakers(&self, _request: SpeakerRequest) {}
}

impl TimeInterface for ReplayerHardwareInterface {
    fn get_now(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl HardwareInterface for ReplayerHardwareInterface {}

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

    let parameters_directory = args().nth(3).unwrap_or("etc/parameters".to_owned());

    let hardware_interface = ReplayerHardwareInterface {
        ids: Ids {
            body_id: "replayer".to_string(),
            head_id: "replayer".to_string(),
        },
    };

    let ids = hardware_interface.get_ids();

    let mut image_extractor = ImageExtractor::new(
        Arc::new(hardware_interface),
        parameters_directory,
        &ids.body_id,
        &ids.head_id,
        replay_path,
    )
    .wrap_err("failed to create replayer")?;

    let vision_top_reader = image_extractor.vision_top_reader();
    let vision_bottom_reader = image_extractor.vision_bottom_reader();

    for (instance_name, reader) in [
        ("VisionTop", vision_top_reader),
        ("VisionBottom", vision_bottom_reader),
    ] {
        let unknown_indices_error_message =
            format!("could not find recording indices for `{instance_name}`");

        let timings: Vec<_> = image_extractor
            .get_recording_indices()
            .get(instance_name)
            .expect(&unknown_indices_error_message)
            .iter()
            .collect();

        for (i, timing) in timings.into_iter().enumerate() {
            let frame = image_extractor
                .get_recording_indices_mut()
                .get_mut(instance_name)
                .map(|index| {
                    index
                        .find_latest_frame_up_to(timing.timestamp)
                        .expect("failed to find latest frame")
                })
                .expect(&unknown_indices_error_message);

            if let Some(frame) = frame {
                image_extractor
                    .replay(instance_name, frame.timing.timestamp, &frame.data)
                    .expect("failed to replay frame");
            }

            let database = reader.next();
            let output_file = output_folder.join(format!("{:05}_{instance_name}.png", i));
            database
                .main_outputs
                .image
                .save_to_ycbcr_444_file(output_file)
                .expect("failed to write file");
        }
    }

    Ok(())
}
