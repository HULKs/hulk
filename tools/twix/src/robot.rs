use std::time::{Duration, SystemTime};

use bincode::deserialize;
use color_eyre::{
    Report, Result,
    eyre::{Context, OptionExt, eyre},
};
use log::error;
use serde_json::Value;
use tokio::{
    runtime::{Builder, Runtime},
    select, spawn,
};

use communication::{
    client::{
        Client, ClientHandle, PathsEvent, Status, SubscriptionHandle,
        protocol::{self, SubscriptionEvent},
    },
    messages::{Path, TextOrBinary},
};
use hula_types::hardware::Ids;
use parameters::{directory::Scope, json::nest_value_at_path};
use repository::Repository;

use crate::{
    change_buffer::{Change, ChangeBuffer, ChangeBufferHandle},
    value_buffer::{Buffer, BufferHandle, Datum},
};

pub struct Robot {
    runtime: Runtime,
    client: ClientHandle,
    write_client: ClientHandle,
    repository: Option<Repository>,
}

impl Robot {
    pub fn new(address: String, repository: Option<Repository>) -> Self {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

        let (client, handle) = Client::new(address.clone());
        let (write_client, write_handle) = Client::new(address);
        runtime.spawn(client.run());
        runtime.spawn(write_client.run());

        Self {
            runtime,
            client: handle,
            write_client: write_handle,
            repository,
        }
    }

    pub fn connect(&self) {
        let client = self.client.clone();
        self.runtime.spawn(async move { client.connect().await });
        let write_client = self.write_client.clone();
        self.runtime
            .spawn(async move { write_client.connect().await });
    }

    pub fn disconnect(&self) {
        let client = self.client.clone();
        self.runtime.spawn(async move { client.disconnect().await });
        let write_client = self.write_client.clone();
        self.runtime
            .spawn(async move { write_client.disconnect().await });
    }

    pub fn connection_status(&self) -> Status {
        self.runtime.block_on(async {
            merge_connection_status(self.client.status().await, self.write_client.status().await)
        })
    }

    pub fn set_address(&self, address: String) {
        let client = self.client.clone();
        let read_address = address.clone();
        self.runtime.spawn(async move {
            client.set_address(read_address).await;
        });
        let write_client = self.write_client.clone();
        self.runtime.spawn(async move {
            write_client.set_address(address).await;
        });
    }

    pub fn latest_paths(&self) -> PathsEvent {
        self.client.paths.borrow().clone()
    }

    pub fn blocking_read<T>(
        &self,
        path: impl Into<Path>,
    ) -> Result<(SystemTime, T), color_eyre::eyre::Error>
    where
        for<'de> T: serde::Deserialize<'de> + Send + Sync + 'static,
    {
        let (timestamp, bytes) = self
            .runtime
            .block_on(self.client.read_binary(path.into()))?;
        let value = deserialize(&bytes)?;
        Ok((timestamp, value))
    }

    pub fn subscribe_json(&self, path: impl Into<Path>) -> BufferHandle<Value> {
        self.subscribe_buffered_json(path, Duration::ZERO)
    }

    pub fn subscribe_buffered_json(
        &self,
        path: impl Into<Path>,
        history: Duration,
    ) -> BufferHandle<Value> {
        let path = path.into();
        let _guard = self.runtime.enter();
        let (task, buffer) = Buffer::new(history);
        let client = self.client.clone();
        spawn(async move {
            let subscription = client.subscribe_text(path).await;
            map_subscription(task, subscription, |datum| -> Result<_, Report> {
                let datum = datum.map_err(|error| eyre!("{error:#}"))?;
                Ok(Datum {
                    timestamp: datum.timestamp,
                    value: datum.value.clone(),
                })
            })
            .await;
        });
        buffer
    }

    pub fn subscribe_changes_json(&self, path: impl Into<Path>) -> ChangeBufferHandle<Value> {
        let path = path.into();
        let _guard = self.runtime.enter();
        let (task, buffer) = ChangeBuffer::new();
        let client = self.client.clone();
        spawn(async move {
            let subscription = client.subscribe_text(path).await;
            task.map(subscription, |datum| -> Result<_, Report> {
                let datum = datum.map_err(|error| eyre!("{error:#}"))?;
                Ok(Change {
                    timestamp: datum.timestamp,
                    value: datum.value.clone(),
                })
            })
            .await;
        });
        buffer
    }

    pub fn subscribe_value<T>(&self, path: impl Into<Path>) -> BufferHandle<T>
    where
        for<'de> T: serde::Deserialize<'de> + Send + Sync + 'static,
    {
        self.subscribe_buffered_value(path, Duration::ZERO)
    }

    pub fn subscribe_buffered_value<T>(
        &self,
        path: impl Into<Path>,
        history: Duration,
    ) -> BufferHandle<T>
    where
        for<'de> T: serde::Deserialize<'de> + Send + Sync + 'static,
    {
        let path = path.into();
        let _guard = self.runtime.enter();
        let (task, buffer) = Buffer::new(history);
        let client = self.client.clone();
        spawn(async move {
            let subscription = client.subscribe_binary(path).await;
            map_subscription(task, subscription, |datum| -> Result<_, Report> {
                let datum = datum.map_err(|error| eyre!("protocol: {error:#}"))?;
                Ok(Datum {
                    timestamp: datum.timestamp,
                    value: deserialize(datum.value).wrap_err("bincode deserialization failed")?,
                })
            })
            .await;
        });
        buffer
    }

    pub fn write(&self, path: impl Into<Path>, value: TextOrBinary) {
        let client = self.write_client.clone();
        let path = path.into();
        self.runtime.spawn(async move {
            if let Err(error) = client.write(path, value).await {
                error!("{error:#}")
            }
        });
    }

    pub fn on_change(&self, callback: impl Fn() + Send + Sync + 'static) {
        let _guard = self.runtime.enter();
        self.client.on_change(callback)
    }

    pub fn store_parameters(&self, path: &str, value: Value, scope: Scope) -> Result<()> {
        let client = self.client.clone();
        let parameters_root = self
            .repository
            .as_ref()
            .ok_or_eyre("repository not available, cannot store parameters")?
            .root
            .join("etc/parameters/");
        self.runtime.block_on(async {
            if let Err(error) = store_parameters(&client, path, value, scope, parameters_root).await
            {
                error!("{error:#}")
            }
        });
        Ok(())
    }
}

fn merge_connection_status(read_status: Status, write_status: Status) -> Status {
    match (read_status, write_status) {
        (Status::Connected, Status::Connected) => Status::Connected,
        (Status::Connecting, _) | (_, Status::Connecting) => Status::Connecting,
        _ => Status::Disconnected,
    }
}

async fn map_subscription<T, U, E>(
    buffer: Buffer<T, E>,
    mut subscription: SubscriptionHandle<U>,
    op: impl Fn(Result<Datum<&U>, &protocol::Error>) -> Result<Datum<T>, E> + Send + Sync + 'static,
) {
    loop {
        select! {
            maybe_event = subscription.receiver.recv() => {
                let Ok(event) = maybe_event else {
                    break;
                };

                let datum = match event.as_ref() {
                    SubscriptionEvent::Successful { timestamp, value }
                    | SubscriptionEvent::Update { timestamp, value } => Ok(Datum {
                        timestamp: *timestamp,
                        value,
                    }),
                    SubscriptionEvent::Failure { error } => Err(error),
                };

                match op(datum) {
                    Ok(datum) => buffer.push(datum).await,
                    Err(error) => buffer.send_error(error),
                }
            }
            _ = buffer.closed() => break,
        }
    }
}

async fn store_parameters(
    client: &ClientHandle,
    path: &str,
    value: Value,
    scope: Scope,
    parameters_root: impl AsRef<std::path::Path>,
) -> Result<()> {
    let (_, bytes) = client.read_binary("hardware_ids").await?;
    let ids: Ids = bincode::deserialize(&bytes).wrap_err("bincode deserialization failed")?;
    let parameters = nest_value_at_path(path, value);
    parameters::directory::serialize(&parameters, scope, path, parameters_root, &ids)
        .wrap_err("serialization failed")?;
    Ok(())
}
