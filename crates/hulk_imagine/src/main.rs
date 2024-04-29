#![recursion_limit = "256"]
use std::fs::create_dir_all;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env::args, path::PathBuf, sync::Arc};

use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use hardware::{CameraInterface, PathsInterface, TimeInterface};
use types::{camera_position::CameraPosition, hardware::Paths, ycbcr422_image::YCbCr422Image};

use crate::execution::Replayer;

pub trait HardwareInterface: CameraInterface + PathsInterface + TimeInterface {}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

struct ImageExtractorHardwareInterface;

impl CameraInterface for ImageExtractorHardwareInterface {
    fn read_from_camera(&self, _camera_position: CameraPosition) -> Result<YCbCr422Image> {
        panic!("Replayer cannot produce data from hardware")
    }
}

impl PathsInterface for ImageExtractorHardwareInterface {
    fn get_paths(&self) -> Paths {
        Paths {
            motions: "etc/motions".into(),
            neural_networks: "etc/neural_networks".into(),
            sounds: "etc/sounds".into(),
        }
    }
}

impl TimeInterface for ImageExtractorHardwareInterface {
    fn get_now(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl HardwareInterface for ImageExtractorHardwareInterface {}

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
    let id = "replayer".to_string();

    let mut replayer = Replayer::new(
        Arc::new(ImageExtractorHardwareInterface),
        parameters_directory,
        id.clone(),
        id,
        replay_path,
    )
    .wrap_err("failed to create image extractor")?;

    let vision_top_reader = replayer.vision_top_reader();
    let vision_bottom_reader = replayer.vision_bottom_reader();

    for (instance_name, reader) in [
        ("VisionTop", vision_top_reader),
        ("VisionBottom", vision_bottom_reader),
    ] {
        let output_folder = &output_folder.join(instance_name);
        create_dir_all(output_folder).expect("failed to create output folder");

        let unknown_indices_error_message =
            format!("could not find recording indices for `{instance_name}`");
        let timings: Vec<_> = replayer
            .get_recording_indices()
            .get(instance_name)
            .expect(&unknown_indices_error_message)
            .iter()
            .collect();

        for timing in timings {
            let frame = replayer
                .get_recording_indices_mut()
                .get_mut(instance_name)
                .map(|index| {
                    index
                        .find_latest_frame_up_to(timing.timestamp)
                        .expect("failed to find latest frame")
                })
                .expect(&unknown_indices_error_message);

            if let Some(frame) = frame {
                replayer
                    .replay(instance_name, frame.timing.timestamp, &frame.data)
                    .expect("failed to replay frame");

                let database = reader.next();
                let output_file = output_folder.join(format!(
                    "{}.png",
                    frame
                        .timing
                        .timestamp
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                ));
                database
                    .main_outputs
                    .image
                    .save_to_ycbcr_444_file(output_file)
                    .expect("failed to write file");
            }
        }
    }

    Ok(())
}
