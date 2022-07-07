use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
};

use log::error;
use serde_json::Value;
use serialize_hierarchy::{HierarchyType, SerializeHierarchy};
use tokio::{
    spawn,
    sync::{
        mpsc::{self, Receiver},
        oneshot,
    },
    task::JoinHandle,
};

use crate::framework::Configuration;

use super::{
    receiver::respond_or_log_error,
    sender::{Message, Payload},
    ChannelsForParameters,
};

#[derive(Debug)]
pub enum Request {
    GetParameterHierarchy {
        response_sender: oneshot::Sender<HierarchyType>,
    },
    SubscribeParameter {
        client: SocketAddr,
        path: String,
        response_sender: oneshot::Sender<Result<(), &'static str>>,
        parameter_sender: mpsc::Sender<Message>,
    },
    UnsubscribeParameter {
        client: SocketAddr,
        path: String,
        response_sender: oneshot::Sender<Result<(), &'static str>>,
    },
    UnsubscribeEverything {
        client: SocketAddr,
    },
    UpdateParameter {
        path: String,
        data: Value,
        response_sender: oneshot::Sender<Result<(), &'static str>>,
    },
}

pub async fn parameter_modificator(
    mut request_receiver: Receiver<Request>,
    initial_configuration: Configuration,
    channels: ChannelsForParameters,
) -> JoinHandle<()> {
    spawn(async move {
        let parameter_hierarchy = Configuration::get_hierarchy();
        let mut configuration = initial_configuration;
        let mut subscriptions = HashMap::new();
        while let Some(request) = request_receiver.recv().await {
            handle_request(
                request,
                &parameter_hierarchy,
                &mut configuration,
                &mut subscriptions,
                &channels,
            )
            .await;
        }
    })
}

#[derive(Debug)]
struct Peer {
    parameter_sender: mpsc::Sender<Message>,
    paths: HashSet<String>,
}

async fn handle_request(
    request: Request,
    parameter_hierarchy: &HierarchyType,
    configuration: &mut Configuration,
    subscriptions: &mut HashMap<SocketAddr, Peer>,
    channels: &ChannelsForParameters,
) {
    match request {
        Request::GetParameterHierarchy { response_sender } => {
            respond_or_log_error(response_sender, parameter_hierarchy.clone());
        }
        Request::SubscribeParameter {
            client,
            path,
            response_sender,
            parameter_sender,
        } => {
            handle_subscribe_parameter(
                client,
                path,
                response_sender,
                parameter_sender,
                configuration,
                subscriptions,
            )
            .await
        }
        Request::UnsubscribeParameter {
            client,
            path,
            response_sender,
        } => {
            handle_unsubscribe_parameter(client, path, response_sender, subscriptions).await;
        }
        Request::UnsubscribeEverything { client } => {
            handle_unsubscribe_everything(client, subscriptions).await;
        }
        Request::UpdateParameter {
            path,
            data,
            response_sender,
        } => {
            handle_update_parameter(
                path,
                data,
                response_sender,
                configuration,
                subscriptions,
                channels,
            )
            .await;
        }
    }
}

async fn handle_subscribe_parameter(
    client: SocketAddr,
    path: String,
    response_sender: oneshot::Sender<Result<(), &'static str>>,
    parameter_sender: mpsc::Sender<Message>,
    configuration: &mut Configuration,
    subscriptions: &mut HashMap<SocketAddr, Peer>,
) {
    if !Configuration::exists(&path) {
        respond_or_log_error(response_sender, Err("Path does not exist"));
        return;
    }
    let peer = subscriptions.entry(client).or_insert_with(|| Peer {
        parameter_sender: parameter_sender.clone(),
        paths: Default::default(),
    });
    let response = match peer.paths.insert(path.clone()) {
        true => Ok(()),
        false => Err("Already subscribed"),
    };
    respond_or_log_error(response_sender, response);

    let data = match configuration.serialize_hierarchy(&path) {
        Ok(data) => data,
        Err(error) => {
            error!("Failed to serialize by path: {:?}", error);
            return;
        }
    };
    send_parameter_to_client(path, data, parameter_sender).await;
}

async fn handle_unsubscribe_parameter(
    client: SocketAddr,
    path: String,
    response_sender: oneshot::Sender<Result<(), &'static str>>,
    subscriptions: &mut HashMap<SocketAddr, Peer>,
) {
    let peer = match subscriptions.get_mut(&client) {
        Some(paths) => paths,
        None => {
            respond_or_log_error(
                response_sender,
                Err("Not subscribed (client not registered)"),
            );
            return;
        }
    };
    if !peer.paths.remove(&path) {
        respond_or_log_error(response_sender, Err("Not subscribed (path not registered)"));
        return;
    }
    if peer.paths.is_empty() {
        subscriptions.remove(&client);
    }
    respond_or_log_error(response_sender, Ok(()));
}

async fn handle_unsubscribe_everything(
    client: SocketAddr,
    subscriptions: &mut HashMap<SocketAddr, Peer>,
) {
    subscriptions.remove(&client);
}

async fn handle_update_parameter(
    path: String,
    data: Value,
    response_sender: oneshot::Sender<Result<(), &'static str>>,
    configuration: &mut Configuration,
    subscriptions: &mut HashMap<SocketAddr, Peer>,
    channels: &ChannelsForParameters,
) {
    if !Configuration::exists(&path) {
        respond_or_log_error(response_sender, Err("Path does not exist"));
        return;
    }
    if let Err(error) = configuration.deserialize_hierarchy(&path, data.clone()) {
        error!("Failed to deserialize by path: {:?}", error);
        respond_or_log_error(response_sender, Err("Failed to deserialize"));
        return;
    }
    {
        let mut configuration_slot = channels.configuration.next();
        *configuration_slot = configuration.clone();
    }
    respond_or_log_error(response_sender, Ok(()));
    if let Err(error) = channels.changed_parameters.send(path.clone()) {
        error!(
            "Failed to send message into channel for changed parameters: {:?}",
            error
        );
    }
    for peer in subscriptions.values() {
        send_parameter_to_client(path.clone(), data.clone(), peer.parameter_sender.clone()).await;
    }
}

async fn send_parameter_to_client(
    path: String,
    data: Value,
    parameter_sender: mpsc::Sender<Message>,
) {
    if let Err(error) = parameter_sender
        .send(Message::Json {
            payload: Payload::ParameterUpdated { path, data },
        })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}
