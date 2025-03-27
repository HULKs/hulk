#![recursion_limit = "256"]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs::{create_dir_all, read_to_string, File};
use std::io::BufWriter;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Parser;
use color_eyre::eyre::{ContextCompat, OptionExt};
use color_eyre::{
    eyre::{Result, WrapErr},
    install,
};
use mcap::records::system_time_to_nanos;
use mcap::{records::Metadata, Attachment};
use rmp_serde::to_vec_named;
use serde_json::from_str;

use hula_types::Ids;
use structs::Parameters;

use crate::execution::Replayer;
use crate::{
    extractor_hardware_interface::{ExtractorHardwareInterface, HardwareInterface},
    mcap_converter::McapConverter,
    write_to_mcap::write_to_mcap,
};

mod extractor_hardware_interface;
mod mcap_converter;
mod serializer;
mod write_to_mcap;

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

#[derive(Parser, Debug)]
#[clap(name = "imagine")]
struct CommandlineArguments {
    #[arg(required = true)]
    replay_path_string: String,
    #[arg(required = true)]
    output_folder: String,
    parameters_directory: Option<String>,
}

fn main() -> Result<()> {
    install()?;

    let arguments = CommandlineArguments::parse();

    let output_folder = PathBuf::from(arguments.output_folder);
    let parameters_directory = arguments
        .parameters_directory
        .unwrap_or(arguments.replay_path_string.clone());

    let ids = Ids {
        body_id: "replayer".into(),
        head_id: "replayer".into(),
    };

    let replay_path = arguments.replay_path_string.clone();
    let replay_path = Path::new(&replay_path);

    let parameters = read_to_string(replay_path.join("default.json"))
        .wrap_err("failed to open framework parameters")?;

    let ip_address = replay_path
        .parent()
        .and_then(|path| path.file_name())
        .wrap_err("expected replay path to have parent directory")?
        .to_str()
        .wrap_err("replay directory name is no valid UTF-8")?;

    let mut replayer = Replayer::new(
        Arc::new(ExtractorHardwareInterface),
        parameters_directory,
        ids,
        arguments.replay_path_string,
    )
    .wrap_err("failed to create image extractor")?;

    replayer
        .audio_subscriptions_sender
        .borrow_mut()
        .insert("additional_outputs".to_string());
    replayer
        .control_subscriptions_sender
        .borrow_mut()
        .insert("additional_outputs".to_string());
    replayer
        .spl_network_subscriptions_sender
        .borrow_mut()
        .insert("additional_outputs".to_string());
    replayer
        .vision_top_subscriptions_sender
        .borrow_mut()
        .insert("additional_outputs".to_string());
    replayer
        .vision_bottom_subscriptions_sender
        .borrow_mut()
        .insert("additional_outputs".to_string());

    create_dir_all(&output_folder).wrap_err("failed to create output folder")?;

    let output_file = output_folder.join("outputs.mcap");

    let mut mcap_converter =
        McapConverter::from_writer(BufWriter::new(File::create(output_file)?))?;

    let metadata = Metadata {
        name: String::from("robot data"),
        metadata: BTreeMap::from([(String::from("IP address"), String::from(ip_address))]),
    };
    mcap_converter
        .write_metadata(metadata)
        .wrap_err("failed to write metadata")?;

    let framework_start_time = replayer
        .get_recording_indices()
        .get("Control")
        .wrap_err("could not find recording indices for `Control`")?
        .first_timing()
        .ok_or_eyre("first timing does not exist")?
        .timestamp;

    let parameter_data: Parameters =
        from_str(&parameters).wrap_err("failed to parse parameters")?;
    let parameters = to_vec_named(&parameter_data)?;

    let attachment = Attachment {
        log_time: system_time_to_nanos(&framework_start_time),
        create_time: system_time_to_nanos(&framework_start_time),
        name: String::from("parameters"),
        media_type: String::from("MessagePack"),
        data: Cow::Owned(parameters),
    };
    mcap_converter.attach(attachment)?;

    let audio_receiver = replayer.audio_receiver();
    let control_receiver = replayer.control_receiver();
    let spl_network_receiver = replayer.spl_network_receiver();
    let vision_top_receiver = replayer.vision_top_receiver();
    let vision_bottom_receiver = replayer.vision_bottom_receiver();

    write_to_mcap(&mut replayer, "Audio", &mut mcap_converter, audio_receiver)
        .wrap_err("failed to write audio data to mcap")?;
    write_to_mcap(
        &mut replayer,
        "Control",
        &mut mcap_converter,
        control_receiver,
    )
    .wrap_err("failed to write control data to mcap")?;
    write_to_mcap(
        &mut replayer,
        "VisionBottom",
        &mut mcap_converter,
        vision_bottom_receiver,
    )
    .wrap_err("failed to write vision bottom data to mcap")?;
    write_to_mcap(
        &mut replayer,
        "VisionTop",
        &mut mcap_converter,
        vision_top_receiver,
    )
    .wrap_err("failed to write vision top data to mcap")?;
    write_to_mcap(
        &mut replayer,
        "SplNetwork",
        &mut mcap_converter,
        spl_network_receiver,
    )
    .wrap_err("failed to write spl network data to mcap")?;

    mcap_converter.finish()?;

    Ok(())
}
