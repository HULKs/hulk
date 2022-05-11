use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::{
    spawn,
    sync::{mpsc, oneshot},
};

use super::{
    id_tracker::{self, id_tracker},
    manager::{self, manager},
    receiver::receiver,
    requester::{self, requester},
    responder::{self, responder},
    CyclerOutput,
};

pub struct Connection {
    responder: mpsc::Sender<responder::Message>,
    requester: mpsc::Sender<requester::Message>,
    manager: mpsc::Sender<manager::Message>,
    id_tracker: mpsc::Sender<id_tracker::Message>,
}

impl Connection {
    pub async fn connect(address: &str) -> Result<Self> {
        let (ws_stream, _response) = tokio_tungstenite::connect_async(address)
            .await
            .with_context(|| anyhow!("Cannot connect websocket to {address}"))?;
        let (writer, reader) = ws_stream.split();
        let (manager_sender, manager_receiver) = mpsc::channel(1);
        let (requester_sender, requester_receiver) = mpsc::channel(1);
        let (responder_sender, responder_receiver) = mpsc::channel(1);
        let (id_tracker_sender, id_tracker_receiver) = mpsc::channel(1);
        spawn(manager(manager_receiver));
        spawn(requester(requester_receiver, writer));
        spawn(receiver(
            reader,
            responder_sender.clone(),
            manager_sender.clone(),
        ));
        spawn(responder(responder_receiver));
        spawn(id_tracker(id_tracker_receiver));
        Ok(Self {
            responder: responder_sender,
            requester: requester_sender,
            manager: manager_sender,
            id_tracker: id_tracker_sender,
        })
    }

    async fn get_message_id(&self) -> Result<usize> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.id_tracker
            .send(id_tracker::Message::GetId { response_sender })
            .await
            .context("Failed to send to id tracker")?;
        response_receiver
            .await
            .context("Failed to receive from response channel for message id")
    }

    pub async fn subscribe(
        &self,
        output: CyclerOutput,
        output_sender: mpsc::Sender<Value>,
    ) -> Result<()> {
        let message_id = self.get_message_id().await?;
        let (response_sender, response_receiver) = oneshot::channel();
        self.responder
            .send(responder::Message::Wait {
                id: message_id,
                response_sender,
            })
            .await
            .context("Failed to send to responder channel")?;
        let request = requester::Message::SubscribeOutput {
            id: message_id,
            output: output.clone(),
        };
        self.requester.send(request).await?;
        response_receiver
            .await
            .context("Could not receive from response channel")?
            .context("Failed to subscribe output")?;
        self.manager
            .send(manager::Message::SubscribeOutput {
                output,
                output_sender,
            })
            .await
            .context("Failed to send to manager_sender")?;
        Ok(())
    }
}
