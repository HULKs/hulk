use serde_json::Value;
use tokio::{
    spawn,
    sync::{mpsc, oneshot},
};
use uuid::Uuid;

use crate::{
    connector::{self, connector},
    parameter_subscription_manager::{self, parameter_subscription_manager},
    HierarchyType, OutputHierarchy, SubscriberMessage,
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

        spawn(connector(
            connector_receiver,
            connector_sender.clone(),
            output_subscription_manager_sender.clone(),
            parameter_subscription_manager_sender.clone(),
            responder_sender.clone(),
            address,
            connect,
        ));
        spawn(output_subscription_manager(
            output_subscription_manager_receiver,
            output_subscription_manager_sender.clone(),
            id_tracker_sender.clone(),
            responder_sender.clone(),
        ));
        spawn(parameter_subscription_manager(
            parameter_subscription_manager_receiver,
            parameter_subscription_manager_sender.clone(),
            id_tracker_sender,
            responder_sender,
        ));
        spawn(id_tracker(id_tracker_receiver));
        spawn(responder(responder_receiver));

        Self {
            connector: connector_sender,
            output_subscription_manager: output_subscription_manager_sender,
            parameter_subscription_manager: parameter_subscription_manager_sender,
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

    pub async fn subscribe_output(
        &self,
        output: CyclerOutput,
    ) -> (Uuid, mpsc::Receiver<SubscriberMessage>) {
        let (subscriber_sender, subscriber_receiver) = mpsc::channel(10);
        let (response_sender, response_receiver) = oneshot::channel();
        self.output_subscription_manager
            .send(output_subscription_manager::Message::Subscribe {
                output,
                subscriber: subscriber_sender,
                response_sender,
            })
            .await
            .unwrap();
        let uuid = response_receiver.await.unwrap();
        (uuid, subscriber_receiver)
    }

    pub async fn unsubscribe_output(&self, output: CyclerOutput, uuid: Uuid) {
        self.output_subscription_manager
            .send(output_subscription_manager::Message::Unsubscribe { output, uuid })
            .await
            .unwrap();
    }

    pub async fn subscribe_parameter(
        &self,
        path: String,
    ) -> (Uuid, mpsc::Receiver<SubscriberMessage>) {
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

    pub async fn unsubscribe_parameter(&self, path: String, uuid: Uuid) {
        self.parameter_subscription_manager
            .send(parameter_subscription_manager::Message::Unsubscribe { path, uuid })
            .await
            .unwrap();
    }

    pub async fn get_output_hiearchy(&self) -> Option<OutputHierarchy> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.output_subscription_manager
            .send(output_subscription_manager::Message::GetOutputHierarchy { response_sender })
            .await
            .unwrap();
        response_receiver.await.unwrap()
    }

    pub async fn get_parameter_hiearchy(&self) -> Option<HierarchyType> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.parameter_subscription_manager
            .send(
                parameter_subscription_manager::Message::GetParameterHierarchy { response_sender },
            )
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
