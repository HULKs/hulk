use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{cyclers::control::Database, robot::to_player_number, simulator::Frame};
use color_eyre::{
    eyre::{Context, Error},
    Result,
};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use tokio::{net::ToSocketAddrs, select, time::interval};
use tokio_util::sync::CancellationToken;
use types::{
    ball_position::SimulatorBallState, field_dimensions::FieldDimensions, hardware::Ids,
    players::Players,
};

#[derive(Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
struct Parameters {
    selected_frame: usize,
    selected_robot: usize,
    field_dimensions: FieldDimensions,
}

#[derive(Clone, Default, Serialize, PathSerialize, PathIntrospect)]
struct MainOutputs {
    frame_count: usize,
    ball: Option<SimulatorBallState>,
    databases: Players<Option<Database>>,
}

#[derive(Clone, Default, Serialize, PathSerialize, PathIntrospect)]
struct BehaviorSimulatorDatabase {
    main_outputs: MainOutputs,
}

async fn timeline_server(
    keep_running: CancellationToken,
    mut parameters_reader: buffered_watch::Receiver<(SystemTime, Parameters)>,
    mut outputs_writer: buffered_watch::Sender<(SystemTime, BehaviorSimulatorDatabase)>,
    mut control_writer: buffered_watch::Sender<(SystemTime, Database)>,
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

        let (_, parameters) = &*parameters_reader.borrow_and_mark_as_seen();

        {
            let (_, outputs) = &mut *outputs_writer.borrow_mut();
            outputs.main_outputs.frame_count = frames.len();
            let frame = &frames[parameters.selected_frame];
            outputs.main_outputs.ball.clone_from(&frame.ball);
            outputs.main_outputs.databases = frame.robots.clone();
        }

        {
            let (_, control) = &mut *control_writer.borrow_mut();
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
    frames: Vec<Frame>,
    addresses: impl ToSocketAddrs + Send + Sync + 'static,
    keep_running: CancellationToken,
) -> Result<()> {
    let ids = Ids {
        body_id: "behavior_simulator".to_string(),
        head_id: "behavior_simulator".to_string(),
    };
    let parameters_from_disk: Parameters =
        parameters::directory::deserialize("crates/hulk_behavior_simulator", &ids, true)
            .wrap_err("failed to parse initial parameters")?;
    let initial_parameters = parameters_from_disk;
    let (parameters_sender, parameters_receiver) =
        buffered_watch::channel((std::time::SystemTime::now(), initial_parameters));

    let (outputs_sender, outputs_receiver) =
        buffered_watch::channel((UNIX_EPOCH, Default::default()));

    let (subscribed_outputs_sender, _subscribed_outputs_receiver) =
        buffered_watch::channel(Default::default());

    let (control_writer, control_reader) =
        buffered_watch::channel((UNIX_EPOCH, Default::default()));

    let (subscribed_control_writer, _subscribed_control_reader) =
        buffered_watch::channel(Default::default());

    let runtime = tokio::runtime::Runtime::new()?;
    let communication_server = {
        let parameters_receiver = parameters_receiver.clone();
        runtime.block_on(async {
            let mut communication_server = communication::server::Server::default();
            let (parameters_subscriptions, _) = buffered_watch::channel(Default::default());
            communication_server.expose_source(
                "BehaviorSimulator",
                outputs_receiver,
                subscribed_outputs_sender,
            )?;
            communication_server.expose_source(
                "Control",
                control_reader,
                subscribed_control_writer,
            )?;
            communication_server.expose_source(
                "parameters",
                parameters_receiver,
                parameters_subscriptions,
            )?;
            communication_server.expose_sink("parameters", parameters_sender)?;
            Ok::<_, Error>(communication_server)
        })?
    };
    let communication_task = {
        let keep_running = keep_running.clone();
        runtime.spawn(async { communication_server.serve(addresses, keep_running).await })
    };
    {
        runtime.spawn(async {
            timeline_server(
                keep_running,
                parameters_receiver,
                outputs_sender,
                control_writer,
                frames,
            )
            .await
        });
    }

    runtime.block_on(communication_task)??;
    Ok(())
}
