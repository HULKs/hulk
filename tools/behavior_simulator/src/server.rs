use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    robot::to_player_number,
    simulator::{Frame, Simulator},
    state::Ball,
};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use cyclers::control::Database;
use framework::{multiple_buffer_with_slots, Reader, Writer};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use tokio::{net::ToSocketAddrs, select, sync::Notify, time::interval};
use tokio_util::sync::CancellationToken;
use types::{FieldDimensions, Players};

#[derive(Clone, Serialize, Deserialize, SerializeHierarchy)]
struct Configuration {
    selected_frame: usize,
    selected_robot: usize,
    field_dimensions: FieldDimensions,
}

#[derive(Clone, Default, Serialize, Deserialize, SerializeHierarchy)]
struct MainOutputs {
    frame_count: usize,
    ball: Option<Ball>,
    databases: Players<Option<Database>>,
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
            let frame = &frames[parameters.selected_frame];
            outputs.main_outputs.ball = frame.ball.clone();
            outputs.main_outputs.databases = frame.robots.clone();
        }
        outputs_changed.notify_waiters();

        {
            let mut control = control_writer.next();
            *control = to_player_number(parameters.selected_robot)
                .ok()
                .and_then(|player_number| {
                    frames[parameters.selected_frame].robots[player_number].clone()
                })
                .unwrap_or_default();
        }
        control_changed.notify_waiters();
    }
}

pub fn run(
    addresses: Option<impl ToSocketAddrs + Send + Sync + 'static>,
    keep_running: CancellationToken,
    scenario_file: impl AsRef<Path>,
) -> Result<()> {
    let parameter_slots = 3; // 2 for communication writer + 1 reader for timeline_server
    let communication_server = communication::server::Runtime::<Configuration>::start(
        addresses,
        "tools/behavior_simulator",
        "behavior_simulator".to_string(),
        "behavior_simulator".to_string(),
        parameter_slots,
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

    let mut simulator = Simulator::try_new()?;
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
