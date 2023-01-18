use color_eyre::eyre::WrapErr;
use futures_util::{stream::SplitSink, SinkExt};
use log::info;
use tokio::{net::TcpStream, sync::mpsc::Receiver};
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};

use crate::messages::Request;

pub async fn requester(
    mut receiver: Receiver<Request>,
    mut writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Message>,
) {
    while let Some(request) = receiver.recv().await {
        let request = serde_json::to_string(&request)
            .wrap_err("serialization of Request type failed")
            .unwrap();
        writer
            .send(tungstenite::Message::Text(request))
            .await
            .wrap_err("failed to send message to socket")
            .unwrap();
    }
    info!("Dropping requester, closing socket");
    writer.close().await.unwrap();
}
