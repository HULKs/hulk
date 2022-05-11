use anyhow::Context;
use awaitgroup::Worker;
use futures_util::StreamExt;
use tokio::{
    net::TcpStream,
    select, spawn,
    sync::mpsc::{channel, Sender},
};
use tokio_tungstenite::accept_async;
use tokio_util::sync::CancellationToken;

use super::{
    database_subscription_manager, parameter_modificator, receiver::receiver, sender::sender,
};

pub async fn connection(
    stream: TcpStream,
    database_subscription_manager_sender: Sender<database_subscription_manager::Request>,
    parameter_modificator_sender: Sender<parameter_modificator::Request>,
    keep_running: CancellationToken,
    wait_group_worker: Worker,
) -> anyhow::Result<()> {
    let peer_address = stream
        .peer_addr()
        .context("Failed to get peer address of TCP stream")?;
    let websocket_stream = select! {
        websocket_stream = accept_async(stream) => websocket_stream.context("Failed to accept websocket")?,
        _ = keep_running.cancelled() => return Ok(()),
    };
    let (writer, reader) = websocket_stream.split();

    let keep_only_self_running = CancellationToken::new();
    let (message_sender, message_receiver) = channel(1);
    spawn(receiver(
        peer_address,
        reader,
        database_subscription_manager_sender,
        parameter_modificator_sender,
        wait_group_worker.clone(),
        keep_running,
        keep_only_self_running.clone(),
        message_sender,
    ));
    spawn(sender(
        writer,
        wait_group_worker,
        keep_only_self_running,
        message_receiver,
    ));

    Ok(())
}
