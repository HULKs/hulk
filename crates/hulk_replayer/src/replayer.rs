use std::{
    env::args,
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};

use blake3::Hash;
use color_eyre::{
    eyre::{Report, WrapErr},
    Result,
};
use ctrlc::set_handler;
use eframe::run_native;
use framework::Parameters as FrameworkParameters;
use hardware::IdInterface;
use hula_types::hardware::Ids;
use serde_json::from_reader;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::{
    execution::Replayer,
    window::Window,
    worker_thread::{spawn_workers, PlayerState},
    ReplayerHardwareInterface,
};

pub fn replay_identifier(replay_path: impl AsRef<Path>) -> Result<Hash> {
    let mut hasher = blake3::Hasher::new();

    for child in replay_path.as_ref().read_dir()? {
        let path = child?.path();
        let Some(extension) = path.extension() else {
            continue;
        };
        if extension != ".bincode" {
            continue;
        }
        hasher.update_mmap(path)?;
    }

    Ok(hasher.finalize())
}

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

    let file =
        File::open(framework_parameters_path).wrap_err("failed to open framework parameters")?;
    let mut framework_parameters: FrameworkParameters =
        from_reader(file).wrap_err("failed to parse framework parameters")?;
    if framework_parameters.communication_addresses.is_none() {
        let fallback = "127.0.0.1:1337";
        println!("framework.json disabled communication, falling back to {fallback}");
        framework_parameters.communication_addresses = Some(fallback.to_string());
    }

    let hardware_interface = ReplayerHardwareInterface {
        ids: Ids {
            body_id: "replayer".to_string(),
            head_id: "replayer".to_string(),
        },
    };

    let ids = hardware_interface.get_ids();

    let replay_identifier =
        replay_identifier(&replay_path).wrap_err("failed to compute replay identifier")?;

    let replayer = Replayer::new(
        Arc::new(hardware_interface),
        replay_path.clone(),
        ids,
        replay_path,
        framework_parameters.communication_addresses,
        keep_running.clone(),
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

            set_handler({
                let keep_running = keep_running.clone();
                let context = context.clone();
                move || {
                    keep_running.cancel();
                    context.request_repaint();
                }
            })?;

            spawn_workers(
                replayer,
                time_sender.clone(),
                keep_running.clone(),
                move || {
                    context.request_repaint();
                },
            );
            Ok(Box::new(Window::new(
                creation_context,
                replay_identifier,
                indices,
                time_sender,
                keep_running,
            )?))
        }),
    )
    .map_err(|error| Report::msg(error.to_string()))
    .wrap_err("failed to run user interface")
}
