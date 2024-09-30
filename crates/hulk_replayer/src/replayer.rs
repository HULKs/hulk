use std::{env::args, fs::File, path::PathBuf, sync::Arc};

use color_eyre::{
    eyre::{Report, WrapErr},
    Result,
};
use ctrlc::set_handler;
use eframe::run_native;
use framework::Parameters as FrameworkParameters;
use hardware::IdInterface;
use log::info;
use serde_json::from_reader;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use types::hardware::Ids;

use crate::{
    execution::Replayer,
    window::Window,
    worker_thread::{spawn_workers, PlayerState},
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
    let mut framework_parameters: FrameworkParameters =
        from_reader(file).wrap_err("failed to parse framework parameters")?;
    if framework_parameters.communication_addresses.is_none() {
        info!("framework.json disabled communication, falling back to :1337");
        framework_parameters.communication_addresses = Some("[::1]:1337".to_string());
    }

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

    run_native(
        "Replayer",
        Default::default(),
        Box::new(move |creation_context| {
            let (time_sender, _) = watch::channel(PlayerState::default());
            let context = creation_context.egui_ctx.clone();
            spawn_workers(replayer, time_sender.clone(), move || {
                context.request_repaint();
            });
            Box::new(Window::new(indices, time_sender))
        }),
    )
    .map_err(|error| Report::msg(error.to_string()))
    .wrap_err("failed to run user interface")
}
