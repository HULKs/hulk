use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::{eyre::Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use tokio::{net::ToSocketAddrs, select, sync::mpsc::UnboundedReceiver};
use tokio_util::sync::CancellationToken;

use hula_types::hardware::Ids;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use types::{ball_position::SimulatorBallState, players::Players};

use crate::{
    cyclers::control::Database, recorder::Frame, robot::to_player_number, structs::Parameters,
};

#[derive(Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct SimulatorState {
    selected_frame: usize,
    selected_robot: usize,
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
    mut simulator_state_reader: buffered_watch::Receiver<(SystemTime, SimulatorState)>,
    mut outputs_writer: buffered_watch::Sender<(SystemTime, BehaviorSimulatorDatabase)>,
    mut control_writer: buffered_watch::Sender<(SystemTime, Database)>,
    mut frame_receiver: UnboundedReceiver<Frame>,
) {
    let mut frames = Vec::<Frame>::new();

    let progress = ProgressBar::new_spinner();
    progress.set_style(ProgressStyle::with_template("[{elapsed}] {pos} {msg}").unwrap());
    loop {
        select! {
            frame = frame_receiver.recv(), if !frame_receiver.is_closed() => {
                match frame {
                    Some(frame) => {
                        frames.push(frame);
                        progress.inc(1);
                        progress.set_message(format!("{:.0}/s", progress.per_sec()));
                    }
                    None => {
                        if !progress.is_finished() {
                            progress.finish();
                        }
                        continue;
                    }
                }
            }
            _ = simulator_state_reader.wait_for_change() => { }
            _ = keep_running.cancelled() => {
                break
            }
        }

        let (_, simulator_state) = &*simulator_state_reader.borrow_and_mark_as_seen();
        if let Some(frame) = &frames.get(simulator_state.selected_frame) {
            {
                let (time, outputs) = &mut *outputs_writer.borrow_mut();
                outputs.main_outputs.frame_count = frames.len();
                outputs.main_outputs.ball.clone_from(&frame.ball);
                outputs.main_outputs.databases = frame.robots.clone();
                *time = frame.timestamp;
            }
            {
                let (time, control) = &mut *control_writer.borrow_mut();
                *control = to_player_number(simulator_state.selected_robot)
                    .ok()
                    .and_then(|player_number| frame.robots[player_number].clone())
                    .unwrap_or_default();
                *time = frame.timestamp;
            }
        }
    }
}

pub async fn run(
    frame_receiver: UnboundedReceiver<Frame>,
    addresses: impl ToSocketAddrs + Send + Sync + 'static,
    keep_running: CancellationToken,
) -> Result<()> {
    let ids = Ids {
        body_id: "behavior_simulator".to_string(),
        head_id: "behavior_simulator".to_string(),
    };
    let initial_parameters: Parameters =
        parameters::directory::deserialize("etc/parameters", &ids, true)
            .wrap_err("failed to parse initial parameters")?;
    let initial_simulator_state: SimulatorState =
        parameters::directory::deserialize("crates/bevyhavior_simulator", &ids, true)
            .wrap_err("failed to parse initial parameters")?;
    let (parameters_sender, parameters_receiver) =
        buffered_watch::channel((std::time::SystemTime::now(), initial_parameters));

    let (simulator_state_sender, simulator_state_receiver) =
        buffered_watch::channel((std::time::SystemTime::now(), initial_simulator_state));

    let (outputs_sender, outputs_receiver) =
        buffered_watch::channel((UNIX_EPOCH, Default::default()));

    let (subscribed_outputs_sender, _subscribed_outputs_receiver) =
        buffered_watch::channel(Default::default());

    let (control_writer, control_reader) =
        buffered_watch::channel((UNIX_EPOCH, Default::default()));

    let (subscribed_control_writer, _subscribed_control_reader) =
        buffered_watch::channel(Default::default());

    let mut communication_server = communication::server::Server::default();
    let (parameters_subscriptions, _) = buffered_watch::channel(Default::default());
    let (simulator_state_subscriptions, _) = buffered_watch::channel(Default::default());
    communication_server.expose_source(
        "BehaviorSimulator",
        outputs_receiver,
        subscribed_outputs_sender,
    )?;
    communication_server.expose_source("Control", control_reader, subscribed_control_writer)?;
    communication_server.expose_source(
        "parameters",
        parameters_receiver.clone(),
        parameters_subscriptions,
    )?;
    communication_server.expose_source(
        "simulator",
        simulator_state_receiver.clone(),
        simulator_state_subscriptions,
    )?;
    communication_server.expose_sink("parameters", parameters_sender)?;
    communication_server.expose_sink("simulator", simulator_state_sender)?;

    {
        let keep_running = keep_running.clone();
        tokio::spawn(async {
            timeline_server(
                keep_running,
                simulator_state_receiver,
                outputs_sender,
                control_writer,
                frame_receiver,
            )
            .await
        });
    }

    communication_server
        .serve(addresses, keep_running)
        .await
        .context("failed to serve")
}
