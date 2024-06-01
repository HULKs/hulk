use std::{
    path::Path,
    time::{Duration, Instant},
};

use crate::{
    cycler::Database,
    robot::to_player_number,
    simulator::{Frame, Simulator},
    state::Ball,
};
use color_eyre::{eyre::bail, owo_colors::OwoColorize, Result};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use tokio::{net::ToSocketAddrs, select, time::interval};
use tokio_util::sync::CancellationToken;
use types::{field_dimensions::FieldDimensions, players::Players};

#[derive(Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
struct Parameters {
    selected_frame: usize,
    selected_robot: usize,
    field_dimensions: FieldDimensions,
}

#[derive(Clone, Default, Serialize, PathSerialize, PathIntrospect)]
struct MainOutputs {
    frame_count: usize,
    ball: Option<Ball>,
    databases: Players<Option<Database>>,
}

#[derive(Clone, Default, Serialize, PathSerialize, PathIntrospect)]
struct BehaviorSimulatorDatabase {
    main_outputs: MainOutputs,
}

#[allow(clippy::too_many_arguments)]
async fn timeline_server(
    keep_running: CancellationToken,
    mut parameters_reader: buffered_watch::Receiver<Parameters>,
    mut outputs_writer: buffered_watch::Sender<BehaviorSimulatorDatabase>,
    mut control_writer: buffered_watch::Sender<Database>,
    frames: Vec<Frame>,
) {
    // Hack to provide frame count to clients initially.
    // Can be removed if communication sends data for
    // subscribed outputs immediately after subscribing
    let mut interval = interval(Duration::from_secs(1));

    loop {
        select! {
            _ = parameters_reader.wait_for_change() => { }
            _ = interval.tick() => { }
            _ = keep_running.cancelled() => {
                break
            }
        }

        let parameters = parameters_reader.borrow_and_mark_as_seen();

        {
            let mut outputs = outputs_writer.borrow_mut();
            outputs.main_outputs.frame_count = frames.len();
            let frame = &frames[parameters.selected_frame];
            outputs.main_outputs.ball.clone_from(&frame.ball);
            outputs.main_outputs.databases = frame.robots.clone();
        }

        {
            let mut control = control_writer.borrow_mut();
            *control = to_player_number(parameters.selected_robot)
                .ok()
                .and_then(|player_number| {
                    frames[parameters.selected_frame].robots[player_number].clone()
                })
                .unwrap_or_default();
        }
    }
}

pub fn run(
    addresses: Option<impl ToSocketAddrs + Send + Sync + 'static>,
    keep_running: CancellationToken,
    scenario_file: impl AsRef<Path>,
) -> Result<()> {
    let communication_server = communication::server::Runtime::<Parameters>::start(
        addresses,
        "tools/behavior_simulator",
        "behavior_simulator".to_string(),
        "behavior_simulator".to_string(),
        keep_running.clone(),
    )?;

    let (outputs_writer, outputs_reader) = buffered_watch::channel(Default::default());

    let (subscribed_outputs_writer, _subscribed_outputs_reader) =
        buffered_watch::channel(Default::default());

    communication_server.register_cycler_instance(
        "BehaviorSimulator",
        outputs_reader,
        subscribed_outputs_writer,
    );

    let (control_writer, control_reader) = buffered_watch::channel(Default::default());

    let (subscribed_control_writer, _subscribed_control_reader) =
        buffered_watch::channel(Default::default());
    communication_server.register_cycler_instance(
        "Control",
        control_reader,
        subscribed_control_writer,
    );

    let mut simulator = Simulator::try_new()?;
    simulator.execute_script(scenario_file)?;

    let start = Instant::now();
    if let Err(error) = simulator.run() {
        eprintln!("{}", error.bright_red())
    }
    let duration = Instant::now() - start;
    println!("Took {:.2} seconds", duration.as_secs_f32());

    let frames = simulator.frames;

    let runtime = tokio::runtime::Runtime::new()?;
    {
        let parameters_reader = communication_server.get_parameters_receiver();
        runtime.spawn(async {
            timeline_server(
                keep_running,
                parameters_reader,
                outputs_writer,
                control_writer,
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
