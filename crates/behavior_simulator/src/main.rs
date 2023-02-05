use std::{io::stdout, sync::Arc, time::Duration};

use color_eyre::{eyre::bail, install, Result};
use communication::server::Runtime;
use framework::{Reader, Writer};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use tokio::{select, sync::Notify, time::interval};
use tokio_util::sync::CancellationToken;
use types::FieldDimensions;

mod cycler;
mod interfake;
mod robot;
mod state;

fn setup_logger(is_verbose: bool) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}  {:<18}  {:>5}  {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(if is_verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .chain(stdout())
        .apply()?;
    Ok(())
}

#[derive(Clone, Serialize, Deserialize, SerializeHierarchy)]
struct Configuration {
    time: usize,
    field_dimensions: FieldDimensions,
}

#[derive(Clone, Default, Serialize, Deserialize, SerializeHierarchy)]
struct MainOutputs {
    frame_count: usize,
}

#[derive(Clone, Default, Serialize, Deserialize, SerializeHierarchy)]
struct BehaviorDatabase {
    main_outputs: MainOutputs,
}

#[allow(clippy::too_many_arguments)]
async fn timeline_server(
    keep_running: CancellationToken,
    parameters_reader: Reader<Configuration>,
    parameters_changed: Arc<Notify>,
    outputs_writer: Writer<BehaviorDatabase>,
    outputs_changed: Arc<Notify>,
    control_writer: Writer<cycler::Database>,
    control_changed: Arc<Notify>,
    frames: Vec<Vec<cycler::Database>>,
) {
    let mut interval = interval(Duration::from_secs(1));
    loop {
        select! {
            _ = parameters_changed.notified() => { }
            _ = interval.tick() => { }
            _ = keep_running.cancelled() => {
                break
            }
        }
        {
            let mut outputs = outputs_writer.next();
            let mut control = control_writer.next();
            let parameters = parameters_reader.next();

            outputs.main_outputs.frame_count = frames.len();
            *control = frames[parameters.time][0].clone();
        }
        outputs_changed.notify_waiters();
        control_changed.notify_waiters();
    }
}

fn run(keep_running: CancellationToken) -> Result<()> {
    let communication_server = Runtime::<Configuration>::start(
        Some("[::]:1337"),
        "tools/behavior-simulator",
        "behavior_simulator".to_string(),
        "behavior_simulator".to_string(),
        2,
        keep_running.clone(),
    )?;

    let (outputs_writer, outputs_reader) = framework::multiple_buffer_with_slots([
        Default::default(),
        Default::default(),
        Default::default(),
    ]);

    let outputs_changed = std::sync::Arc::new(tokio::sync::Notify::new());
    let (subscribed_outputs_writer, _subscribed_outputs_reader) =
        framework::multiple_buffer_with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
        ]);

    communication_server.register_cycler_instance(
        "BehaviorSimulator",
        outputs_changed.clone(),
        outputs_reader,
        subscribed_outputs_writer,
    );

    let (control_writer, control_reader) = framework::multiple_buffer_with_slots([
        Default::default(),
        Default::default(),
        Default::default(),
    ]);

    let control_changed = std::sync::Arc::new(tokio::sync::Notify::new());
    let (subscribed_control_writer, _subscribed_control_reader) =
        framework::multiple_buffer_with_slots([
            Default::default(),
            Default::default(),
            Default::default(),
        ]);
    communication_server.register_cycler_instance(
        "Control",
        control_changed.clone(),
        control_reader,
        subscribed_control_writer,
    );

    let mut state = state::State::new(1);
    state.stiffen_robots();

    let mut frames = Vec::new();
    for _frame_index in 0..10000 {
        let mut robot_frames = Vec::new();

        state.cycle(Duration::from_millis(12));

        for robot in &state.robots {
            robot_frames.push(robot.database.clone());
        }
        frames.push(robot_frames);
    }

    let runtime = tokio::runtime::Runtime::new()?;
    {
        let parameters_changed = communication_server.get_parameters_changed();
        let parameters_reader = communication_server.get_parameters_reader();
        runtime.spawn(async {
            timeline_server(
                keep_running,
                parameters_reader,
                parameters_changed,
                outputs_writer,
                outputs_changed,
                control_writer,
                control_changed,
                frames,
            )
            .await
        });
    }

    let mut encountered_error = false;
    match communication_server.join() {
        Ok(Err(error)) => {
            encountered_error = true;
            println!("{error:?}");
        }
        Err(error) => {
            encountered_error = true;
            println!("{error:?}");
        }
        _ => {}
    }

    if encountered_error {
        bail!("at least one cycler exited with error");
    }
    Ok(())
}

fn main() -> Result<()> {
    setup_logger(true)?;
    install()?;
    let keep_running = CancellationToken::new();
    {
        let keep_running = keep_running.clone();
        ctrlc::set_handler(move || {
            println!("Cancelling...");
            keep_running.cancel();
        })?;
    }
    run(keep_running)
}
