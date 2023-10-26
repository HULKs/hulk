use std::collections::BTreeSet;

use serde_json::Value;
use tokio::{
    spawn,
    sync::{
        broadcast,
        mpsc::{self, Receiver},
        oneshot, watch,
    },
};
use uuid::Uuid;

use crate::{
    client::{
        connector::{self, connector, ConnectionStatus},
        parameter_subscription_manager::{self, parameter_subscription_manager},
        SubscriberMessage,
    },
    messages::{Fields, Format, Path},
};

use super::{
    id_tracker::id_tracker,
    output_subscription_manager::{self, output_subscription_manager},
    responder::responder,
    CyclerOutput,
};

#[derive(Clone)]
pub struct Communication {
    connector: mpsc::Sender<connector::Message>,
    output_subscription_manager: mpsc::Sender<output_subscription_manager::Message>,
    parameter_subscription_manager: mpsc::Sender<parameter_subscription_manager::Message>,
    update_sender: broadcast::Sender<()>,
    connection_status_update_receiver: watch::Receiver<ConnectionStatus>,
}

impl Communication {
    pub fn new(address: Option<String>, connect: bool) -> Self {
        let (connector_sender, connector_receiver) = mpsc::channel(10);
        let (output_subscription_manager_sender, output_subscription_manager_receiver) =
            mpsc::channel(10);
        let (parameter_subscription_manager_sender, parameter_subscription_manager_receiver) =
            mpsc::channel(10);
        let (id_tracker_sender, id_tracker_receiver) = mpsc::channel(10);
        let (responder_sender, responder_receiver) = mpsc::channel(10);
        let (update_sender, _) = broadcast::channel(10);
        let (connection_status_update_sender, connection_status_update_receiver) =
            watch::channel(ConnectionStatus::Disconnected {
                address: address.clone(),
                connect,
            });

        spawn(connector(
            connector_receiver,
            connector_sender.clone(),
            output_subscription_manager_sender.clone(),
            parameter_subscription_manager_sender.clone(),
            responder_sender.clone(),
            update_sender.clone(),
            connection_status_update_sender,
            address,
            connect,
        ));
        spawn(output_subscription_manager(
            output_subscription_manager_receiver,
            output_subscription_manager_sender.clone(),
            id_tracker_sender.clone(),
            responder_sender.clone(),
            update_sender.clone(),
        ));
        spawn(parameter_subscription_manager(
            parameter_subscription_manager_receiver,
            parameter_subscription_manager_sender.clone(),
            id_tracker_sender,
            responder_sender,
            update_sender.clone(),
        ));
        spawn(id_tracker(id_tracker_receiver));
        spawn(responder(responder_receiver));

        Self {
            connector: connector_sender,
            output_subscription_manager: output_subscription_manager_sender,
            parameter_subscription_manager: parameter_subscription_manager_sender,
            update_sender,
            connection_status_update_receiver,
        }
    }

    pub async fn set_connect(&self, connect: bool) {
        self.connector
            .send(connector::Message::SetConnect(connect))
            .await
            .unwrap();
    }

    pub async fn set_address(&self, address: String) {
        self.connector
            .send(connector::Message::SetAddress(address))
            .await
            .unwrap();
    }

    pub fn subscribe_connection_status_updates(&self) -> watch::Receiver<ConnectionStatus> {
        self.connection_status_update_receiver.clone()
    }

    pub fn subscribe_updates(&self) -> broadcast::Receiver<()> {
        self.update_sender.subscribe()
    }

    pub async fn subscribe_output(
        &self,
        output: CyclerOutput,
        format: Format,
    ) -> (Uuid, Receiver<SubscriberMessage>) {
        let (subscriber_sender, subscriber_receiver) = mpsc::channel(10);
        let (response_sender, response_receiver) = oneshot::channel();
        self.output_subscription_manager
            .send(output_subscription_manager::Message::Subscribe {
                output,
                format,
                subscriber: subscriber_sender,
                response_sender,
            })
            .await
            .unwrap();
        let uuid = response_receiver.await.unwrap();
        (uuid, subscriber_receiver)
    }

    pub async fn unsubscribe_output(&self, uuid: Uuid) {
        self.output_subscription_manager
            .send(output_subscription_manager::Message::Unsubscribe { uuid })
            .await
            .unwrap();
    }

    pub async fn subscribe_parameter(&self, path: String) -> (Uuid, Receiver<SubscriberMessage>) {
        let (subscriber_sender, subscriber_receiver) = mpsc::channel(10);
        let (response_sender, response_receiver) = oneshot::channel();
        self.parameter_subscription_manager
            .send(parameter_subscription_manager::Message::Subscribe {
                path,
                subscriber: subscriber_sender,
                response_sender,
            })
            .await
            .unwrap();
        let uuid = response_receiver.await.unwrap();
        (uuid, subscriber_receiver)
    }

    pub async fn unsubscribe_parameter(&self, uuid: Uuid) {
        self.parameter_subscription_manager
            .send(parameter_subscription_manager::Message::Unsubscribe { uuid })
            .await
            .unwrap();
    }

    pub async fn get_output_fields(&self) -> Option<Fields> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.output_subscription_manager
            .send(output_subscription_manager::Message::GetOutputFields { response_sender })
            .await
            .unwrap();
        response_receiver.await.unwrap()
    }

    pub async fn get_parameter_fields(&self) -> Option<BTreeSet<Path>> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.parameter_subscription_manager
            .send(parameter_subscription_manager::Message::GetFields { response_sender })
            .await
            .unwrap();
        response_receiver.await.unwrap()
    }

    pub async fn update_parameter_value(&self, path: &str, value: Value) {
        self.parameter_subscription_manager
            .send(
                parameter_subscription_manager::Message::UpdateParameterValue {
                    path: path.to_owned(),
                    value,
                },
            )
            .await
            .unwrap();
    }
}
