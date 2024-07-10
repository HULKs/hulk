use std::{env::args, fs::File, path::PathBuf, sync::Arc, time::SystemTime};

use color_eyre::{
    eyre::{Report, WrapErr},
    Result,
};
use ctrlc::set_handler;
use eframe::run_native;
use framework::Parameters as FrameworkParameters;
use hardware::IdInterface;
use serde_json::from_reader;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use types::hardware::Ids;

use crate::{
    execution::Replayer, window::Window, worker_thread::spawn_worker, ReplayerHardwareInterface,
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
        replay_path.clone(),
        ids,
        replay_path,
        framework_parameters.communication_addresses,
        keep_running,
    )
    .wrap_err("failed to create replayer")?;

    let indices = replayer
        .get_recording_indices()
        .into_iter()
        .map(|(name, index)| (name, index.iter().collect()))
        .collect();

    let (time_sender, time_receiver) = watch::channel(SystemTime::UNIX_EPOCH);
    spawn_worker(replayer, time_receiver);

    run_native(
        "Replayer",
        Default::default(),
        Box::new(move |_creation_context| Box::new(Window::new(indices, time_sender))),
    )
    .map_err(|error| Report::msg(error.to_string()))
    .wrap_err("failed to run user interface")
}
