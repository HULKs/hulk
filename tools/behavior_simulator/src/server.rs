use std::{
    collections::HashMap,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    cycler::Database,
    simulator::{Frame, Simulator},
};
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use framework::{multiple_buffer_with_slots, Reader, Writer};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::PlayerNumber;
use tokio::{select, sync::Notify, time::interval};
use tokio_util::sync::CancellationToken;
use types::FieldDimensions;

#[derive(Clone, Serialize, Deserialize, SerializeHierarchy)]
struct Configuration {
    selected_frame: usize,
    selected_robot: usize,
    field_dimensions: FieldDimensions,
}

#[derive(Clone, Default, Serialize, Deserialize, SerializeHierarchy)]
struct MainOutputs {
    frame_count: usize,
    databases: HashMap<PlayerNumber, Database>,
}

#[derive(Clone, Default, Serialize, Deserialize, SerializeHierarchy)]
struct BehaviorSimulatorDatabase {
    main_outputs: MainOutputs,
}

#[allow(clippy::too_many_arguments)]
async fn timeline_server(
    keep_running: CancellationToken,
    parameters_reader: Reader<Configuration>,
    parameters_changed: Arc<Notify>,
    outputs_writer: Writer<BehaviorSimulatorDatabase>,
    outputs_changed: Arc<Notify>,
    control_writer: Writer<Database>,
    control_changed: Arc<Notify>,
    frames: Vec<Frame>,
) {
    // Hack to provide frame count to clients initially.
    // Can be removed if communication sends data for
    // subscribed outputs immediately after subscribing
    let mut interval = interval(Duration::from_secs(1));

    loop {
        select! {
            _ = parameters_changed.notified() => { }
            _ = interval.tick() => { }
            _ = keep_running.cancelled() => {
                break
            }
        }

        let parameters = parameters_reader.next();

        {
            let mut outputs = outputs_writer.next();
            outputs.main_outputs.frame_count = frames.len();
            outputs.main_outputs.databases = frames[parameters.selected_frame].robots.clone();
        }
        outputs_changed.notify_waiters();

        {
            let mut control = control_writer.next();
            *control = parameters
                .selected_robot
                .try_into()
                .ok()
                .and_then(|player_number| {
                    frames[parameters.selected_frame].robots.get(&player_number)
                })
                .cloned()
                .unwrap_or_default();
        }
        control_changed.notify_waiters();
    }
}

pub fn run(keep_running: CancellationToken, scenario_file: impl AsRef<Path>) -> Result<()> {
    let communication_server = communication::server::Runtime::<Configuration>::start(
        Some("[::]:1337"),
        "tools/behavior_simulator",
        "behavior_simulator".to_string(),
        "behavior_simulator".to_string(),
        2,
        keep_running.clone(),
    )?;

    let (outputs_writer, outputs_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);

    let outputs_changed = Arc::new(Notify::new());
    let (subscribed_outputs_writer, _subscribed_outputs_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);

    communication_server.register_cycler_instance(
        "BehaviorSimulator",
        outputs_changed.clone(),
        outputs_reader,
        subscribed_outputs_writer,
    );

    let (control_writer, control_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);

    let control_changed = Arc::new(Notify::new());
    let (subscribed_control_writer, _subscribed_control_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    communication_server.register_cycler_instance(
        "Control",
        control_changed.clone(),
        control_reader,
        subscribed_control_writer,
    );

    let mut simulator = Simulator::new();
    simulator.execute_script(scenario_file)?;

    let start = Instant::now();
    let frames = simulator.run().wrap_err("failed to run simulation")?;
    let duration = Instant::now() - start;
    println!("Took {:.2} seconds", duration.as_secs_f32());

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
