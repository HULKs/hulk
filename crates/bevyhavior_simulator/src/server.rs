use std::{
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

use color_eyre::{
    eyre::{Context, Error},
    Result,
};
use serde::{Deserialize, Serialize};
use tokio::{net::ToSocketAddrs, select, sync::mpsc::UnboundedReceiver};
use tokio_util::sync::CancellationToken;

use hula_types::hardware::Ids;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use types::{
    ball_position::SimulatorBallState, field_dimensions::FieldDimensions, players::Players,
};

use crate::{cyclers::control::Database, recorder::Frame, robot::to_player_number};

#[derive(Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct Parameters {
    selected_frame: usize,
    selected_robot: usize,
    pub field_dimensions: FieldDimensions,
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
    mut frame_receiver: UnboundedReceiver<Frame>,
) {
    let mut frames = Vec::<Frame>::new();

    loop {
        select! {
            frame = frame_receiver.recv() => {
                if let Some(frame) = frame {
                    frames.push(frame);
                    print!("serving {} frames\r", frames.len());
                    std::io::stdout().flush().unwrap();
                }
            }
            _ = parameters_reader.wait_for_change() => { }
            _ = keep_running.cancelled() => {
                break
            }
        }

        let (_, parameters) = &*parameters_reader.borrow_and_mark_as_seen();
        if let Some(frame) = &frames.get(parameters.selected_frame) {
            {
                let (time, outputs) = &mut *outputs_writer.borrow_mut();
                outputs.main_outputs.frame_count = frames.len();
                outputs.main_outputs.ball.clone_from(&frame.ball);
                outputs.main_outputs.databases = frame.robots.clone();
                *time = frame.timestamp;
            }
            {
                let (time, control) = &mut *control_writer.borrow_mut();
                *control = to_player_number(parameters.selected_robot)
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
    let parameters_from_disk: Parameters =
        parameters::directory::deserialize("crates/bevyhavior_simulator", &ids, true)
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

    let communication_server = {
        let mut communication_server = communication::server::Server::default();
        let (parameters_subscriptions, _) = buffered_watch::channel(Default::default());
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
        communication_server.expose_sink("parameters", parameters_sender)?;
        Ok::<_, Error>(communication_server)
    }?;

    {
        let keep_running = keep_running.clone();
        tokio::spawn(async {
            timeline_server(
                keep_running,
                parameters_receiver,
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
