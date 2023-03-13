use color_eyre::eyre::{Result, WrapErr};
use futures_util::{stream::SplitSink, SinkExt};
use log::{error, info};
use serde_json::to_string;
use tokio::{net::TcpStream, sync::mpsc::Receiver};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::messages::Request;

pub async fn requester(
    mut receiver: Receiver<Request>,
    mut writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
) {
    while let Some(request) = receiver.recv().await {
        if let Err(error) = forward_message(request, &mut writer).await {
            error!("{error:?}");
            break;
        }
    }
    info!("Dropping requester, closing socket");
    writer.close().await.unwrap();
}

async fn forward_message(
    request: Request,
    writer: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
) -> Result<()> {
    let request = to_string(&request).wrap_err("serialization of Request type failed")?;
    writer
        .send(Message::Text(request))
        .await
        .wrap_err("failed to send message to socket")
}
