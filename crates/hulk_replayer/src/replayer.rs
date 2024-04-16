use std::{env::args, fs::File, path::PathBuf, sync::Arc};

use color_eyre::{
    eyre::{Report, WrapErr},
    Result,
};
use ctrlc::set_handler;
use eframe::run_native;
use framework::Parameters as FrameworkParameters;
use hardware::IdInterface;
use serde_json::from_reader;
use tokio_util::sync::CancellationToken;
use types::hardware::Ids;

use crate::{
    execution::{RecordingFilePaths, Replayer},
    user_interface::ReplayerApplication,
    ReplayerHardwareInterface,
};

pub fn replayer() -> Result<()> {
    let replay_path = PathBuf::from(
        args()
            .nth(1)
            .expect("expected replay path as first parameter"),
    );
    let framework_parameters_path = args()
        .nth(2)
        .unwrap_or("etc/parameters/framework.json".to_string());
    let keep_running = CancellationToken::new();
    set_handler({
        let keep_running = keep_running.clone();
        move || {
            keep_running.cancel();
        }
    })?;

    let file =
        File::open(framework_parameters_path).wrap_err("failed to open framework parameters")?;
    let framework_parameters: FrameworkParameters =
        from_reader(file).wrap_err("failed to parse framework parameters")?;

    let hardware_interface = ReplayerHardwareInterface {
        ids: Ids {
            body_id: "replayer".to_string(),
            head_id: "replayer".to_string(),
        },
    };

    let ids = hardware_interface.get_ids();

    let replayer = Replayer::new(
        Arc::new(hardware_interface),
        framework_parameters.communication_addresses,
        replay_path.clone(),
        ids.body_id,
        ids.head_id,
        keep_running,
        RecordingFilePaths {
            vision_top: replay_path.join("VisionTop.bincode"),
            vision_bottom: replay_path.join("VisionBottom.bincode"),
            control: replay_path.join("Control.bincode"),
            spl_network: replay_path.join("SplNetwork.bincode"),
            audio: replay_path.join("Audio.bincode"),
        },
    )
    .wrap_err("failed to create replayer")?;

    // dbg!(recording_indices);

    // let start = replayer
    //     .first_timestamp()
    //     .expect("first timestamp is required");
    // let end = replayer
    //     .last_timestamp()
    //     .expect("last timestamp is required");

    run_native(
        "Replayer",
        Default::default(),
        Box::new(move |_creation_context| Box::new(ReplayerApplication::new(replayer))),
    )
    .map_err(|error| Report::msg(error.to_string()))
    .wrap_err("failed to run user interface")
}
