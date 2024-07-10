use std::{
    collections::{HashMap, HashSet},
    time::SystemTime,
};

use bincode::{DefaultOptions, Options};
use log::error;
use path_serde::PathSerialize;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use tokio::{
    select,
    sync::{mpsc, oneshot},
    task::{yield_now, JoinSet},
};

use crate::messages::{Format, Path, RequestId, TextOrBinary};

use super::{acceptor::ClientId, connection::ConnectionHandle};

#[derive(Debug, PartialEq, Eq)]
pub struct Update {
    pub timestamp: SystemTime,
    pub texts: HashMap<RequestId, Result<Value, String>>,
    pub binaries: HashMap<RequestId, Result<Vec<u8>, String>>,
}

#[derive(Debug)]
pub struct SubscriptionHandle {
    // TODO: use subscription handle to start subscription, if the successful subscription response
    // is notified to the client
    _unsubscribe: oneshot::Sender<()>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("serialization failed")]
    TextSerialization(#[source] path_serde::serialize::Error<serde_json::Error>),
    #[error("serialization failed")]
    BinarySerialization(#[source] path_serde::serialize::Error<bincode::Error>),
    #[error("duplicate subscription with id `{0}`")]
    DuplicateSubscription(RequestId),
}

pub enum Event {
    Read {
        path: Path,
        format: Format,
        return_sender: oneshot::Sender<Result<(SystemTime, TextOrBinary), Error>>,
    },
    Subscribe {
        path: Path,
        format: Format,
        client: ConnectionHandle,
        id: RequestId,
        return_sender:
            oneshot::Sender<Result<(SubscriptionHandle, SystemTime, TextOrBinary), Error>>,
    },
}

struct Unsubscribe {
    client: ClientId,
    id: RequestId,
}

#[derive(Debug)]
pub struct SourceHandle {
    command_sender: mpsc::Sender<Event>,
}

impl SourceHandle {
    pub async fn read(
        &self,
        path: impl Into<Path>,
        format: Format,
    ) -> Result<(SystemTime, TextOrBinary), Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.command_sender
            .send(Event::Read {
                path: path.into(),
                format,
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub async fn subscribe(
        &self,
        path: impl Into<Path>,
        format: Format,
        client: ConnectionHandle,
        id: RequestId,
    ) -> Result<(SubscriptionHandle, SystemTime, TextOrBinary), Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.command_sender
            .send(Event::Subscribe {
                path: path.into(),
                format,
                client,
                id,
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }
}

#[derive(Debug)]
struct Subscription {
    path: Path,
    format: Format,
}

#[derive(Debug)]
struct ClientSubscriptions {
    client: ConnectionHandle,
    subscriptions: HashMap<RequestId, Subscription>,
}

impl ClientSubscriptions {
    fn new(client: ConnectionHandle) -> Self {
        Self {
            client,
            subscriptions: HashMap::new(),
        }
    }
}

struct SerializationCache {
    timestamp: SystemTime,
    values: HashMap<Path, Result<Value, String>>,
    bytes: HashMap<Path, Result<Vec<u8>, String>>,
}

pub struct Source<T> {
    command_receiver: mpsc::Receiver<Event>,
    data: buffered_watch::Receiver<(SystemTime, T)>,
    client_subscriptions: HashMap<ClientId, ClientSubscriptions>,
    subscriptions_sender: buffered_watch::Sender<HashSet<Path>>,
    unsubscriptions: JoinSet<Unsubscribe>,
}

impl<T> Source<T>
where
    T: Serialize + PathSerialize,
{
    pub fn new(
        data: buffered_watch::Receiver<(SystemTime, T)>,
        subscriptions_sender: buffered_watch::Sender<HashSet<Path>>,
    ) -> (Self, SourceHandle) {
        let (command_sender, command_receiver) = mpsc::channel(1);
        let task = Self {
            command_receiver,
            data,
            client_subscriptions: HashMap::new(),
            subscriptions_sender,
            unsubscriptions: JoinSet::new(),
        };
        let handle = SourceHandle { command_sender };
        (task, handle)
    }

    pub async fn run(mut self) {
        loop {
            select! {
                maybe_command = self.command_receiver.recv() => {
                    match maybe_command {
                        Some(command) => self.handle_command(command),
                        None => break,
                    }
                },
                Some(Ok(Unsubscribe { client, id })) = self.unsubscriptions.join_next() => {
                    self.unsubscribe(client, id);
                },
                Ok(()) = self.data.wait_for_change() => {
                    self.handle_update().await;
                },
            }
        }
    }

    fn handle_command(&mut self, command: Event) {
        match command {
            Event::Read {
                path,
                format,
                return_sender,
            } => {
                let response = self.read(&path, format);
                let _ = return_sender.send(response);
            }
            Event::Subscribe {
                path,
                format,
                client,
                id,
                return_sender,
            } => {
                let response = self.subscribe(path, format, client, id);
                let _ = return_sender.send(response);
            }
        }
    }

    fn read(&mut self, path: &Path, format: Format) -> Result<(SystemTime, TextOrBinary), Error> {
        let (timestamp, value) = {
            let (timestamp, data) = &*self.data.borrow();
            let value = match format {
                Format::Text => {
                    let value = serialize_as_text(data, path)?;
                    TextOrBinary::Text(value)
                }
                Format::Binary => {
                    let bytes = serialize_as_binary(data, path)?;
                    TextOrBinary::Binary(bytes)
                }
            };
            (*timestamp, value)
        };
        Ok((timestamp, value))
    }

    fn subscribe(
        &mut self,
        path: String,
        format: Format,
        client: ConnectionHandle,
        id: usize,
    ) -> Result<(SubscriptionHandle, SystemTime, TextOrBinary), Error> {
        let client_id = client.id();

        if let Some(subscriptions) = self.client_subscriptions.get(&client_id) {
            if subscriptions.subscriptions.contains_key(&id) {
                return Err(Error::DuplicateSubscription(id));
            }
        }

        let (timestamp, value) = {
            let (timestamp, data) = &*self.data.borrow();
            let value = match format {
                Format::Text => {
                    let value = serialize_as_text(data, &path)?;
                    TextOrBinary::Text(value)
                }
                Format::Binary => {
                    let bytes = serialize_as_binary(data, &path)?;
                    TextOrBinary::Binary(bytes)
                }
            };
            (*timestamp, value)
        };

        let subscription = Subscription { path, format };
        self.client_subscriptions
            .entry(client_id)
            .or_insert_with(|| ClientSubscriptions::new(client))
            .subscriptions
            .insert(id, subscription);

        let subscriptions = self.collect_subscriptions();
        *self.subscriptions_sender.borrow_mut() = subscriptions;

        let (unsubscribe_sender, unsubscribe_receiver) = oneshot::channel();
        let handle = SubscriptionHandle {
            _unsubscribe: unsubscribe_sender,
        };
        self.unsubscriptions.spawn(async move {
            let _ = unsubscribe_receiver.await;
            Unsubscribe {
                client: client_id,
                id,
            }
        });
        Ok((handle, timestamp, value))
    }

    fn unsubscribe(&mut self, client: ClientId, id: RequestId) {
        let subscriptions = &mut self
            .client_subscriptions
            .get_mut(&client)
            .expect("client to exist")
            .subscriptions;
        subscriptions.remove(&id).expect("subscription to exist");
        if subscriptions.is_empty() {
            self.client_subscriptions.remove(&client);
        }
        let subscriptions = self.collect_subscriptions();
        *self.subscriptions_sender.borrow_mut() = subscriptions;
    }

    async fn handle_update(&mut self) {
        let cache = self.serialize_subscribed();
        for client_subscriptions in self.client_subscriptions.values() {
            let mut texts = HashMap::new();
            let mut binaries = HashMap::new();
            for (id, subscription) in &client_subscriptions.subscriptions {
                match subscription.format {
                    Format::Text => {
                        let value = cache.values[&subscription.path].clone();
                        texts.insert(*id, value);
                    }
                    Format::Binary => {
                        let bytes = cache.bytes[&subscription.path].clone();
                        binaries.insert(*id, bytes);
                    }
                };
            }
            let update = Update {
                timestamp: cache.timestamp,
                texts,
                binaries,
            };
            client_subscriptions.client.try_send_update(update);
        }
        yield_now().await;
    }

    fn serialize_subscribed(&mut self) -> SerializationCache {
        let mut serialized_values = HashMap::new();
        let mut serialized_bytes = HashMap::new();

        let (timestamp, data) = &*self.data.borrow_and_mark_as_seen();

        for subscription in self
            .client_subscriptions
            .values()
            .flat_map(|map| map.subscriptions.values())
        {
            match subscription.format {
                Format::Text => {
                    let value = serialize_as_text(data, &subscription.path)
                        .map_err(|error| error.to_string());
                    serialized_values.insert(subscription.path.clone(), value);
                }
                Format::Binary => {
                    let bytes = serialize_as_binary(data, &subscription.path)
                        .map_err(|error| error.to_string());
                    serialized_bytes.insert(subscription.path.clone(), bytes);
                }
            }
        }
        SerializationCache {
            timestamp: *timestamp,
            values: serialized_values,
            bytes: serialized_bytes,
        }
    }

    fn collect_subscriptions(&self) -> HashSet<String> {
        self.client_subscriptions
            .values()
            .flat_map(|map| map.subscriptions.values())
            .map(|subscription| subscription.path.clone())
            .collect()
    }
}

fn serialize_as_text<T>(data: &T, path: &Path) -> Result<Value, Error>
where
    T: Serialize + PathSerialize,
{
    let serializer = serde_json::value::Serializer;
    let value = if path.is_empty() {
        data.serialize(serializer)
            .map_err(path_serde::serialize::Error::SerializationFailed)
    } else {
        data.serialize_path(path, serializer)
    }
    .map_err(Error::TextSerialization)?;
    Ok(value)
}

fn serialize_as_binary<T>(data: &T, path: &Path) -> Result<Vec<u8>, Error>
where
    T: Serialize + PathSerialize,
{
    let mut bytes = Vec::new();
    let options = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();
    let mut serializer = bincode::Serializer::new(&mut bytes, options);
    if path.is_empty() {
        data.serialize(&mut serializer)
            .map_err(path_serde::serialize::Error::SerializationFailed)
            .map_err(Error::BinarySerialization)?;
    } else {
        data.serialize_path(path, &mut serializer)
            .map_err(Error::BinarySerialization)?;
    }
    Ok(bytes)
}
