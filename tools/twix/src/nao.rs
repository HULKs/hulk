use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use bincode::deserialize;
use color_eyre::{
    eyre::{eyre, Context, OptionExt},
    Report, Result,
};
use communication::{
    client::{Client, ConnectionHandle, PathsEvent, Status},
    messages::{Path, TextOrBinary},
};
use log::{error, warn};
use parameters::{directory::Scope, json::nest_value_at_path};
use repository::{get_repository_root, Repository};
use serde_json::Value;
use tokio::{
    runtime::{Builder, Runtime},
    spawn,
};
use types::hardware::Ids;

use crate::value_buffer::{Buffer, BufferHandle, Datum};

pub struct Nao {
    runtime: Runtime,
    connection: ConnectionHandle,
    repository: Option<Repository>,
}

impl Nao {
    pub fn new(address: String) -> Self {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

        let (connection, handle) = Client::new(address);
        runtime.spawn(connection.run());

        let repository = match runtime.block_on(get_repository_root()) {
            Ok(root) => Some(Repository::new(root)),
            Err(error) => {
                warn!("{error:#}");
                None
            }
        };

        Self {
            runtime,
            connection: handle,
            repository,
        }
    }

    pub fn connect(&self) {
        let connection = self.connection.clone();
        self.runtime
            .spawn(async move { connection.connect().await });
    }

    pub fn disconnect(&self) {
        let connection = self.connection.clone();
        self.runtime
            .spawn(async move { connection.disconnect().await });
    }

    pub fn connection_status(&self) -> Status {
        self.runtime
            .block_on(async { self.connection.status().await })
    }

    pub fn set_address(&self, address: String) {
        let connection = self.connection.clone();
        self.runtime.spawn(async move {
            connection.set_address(address).await;
        });
    }

    pub fn latest_paths(&self) -> PathsEvent {
        self.connection.paths.borrow().clone()
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
            .block_on(self.connection.read_binary(path.into()))?;
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
        let connection = self.connection.clone();
        spawn(async move {
            let subscription = connection.subscribe_text(path).await;
            task.map(subscription, |datum| -> Result<_, Report> {
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
        let connection = self.connection.clone();
        spawn(async move {
            let subscription = connection.subscribe_binary(path).await;
            task.map(subscription, |datum| -> Result<_, Report> {
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
        let connection = self.connection.clone();
        let path = path.into();
        self.runtime.spawn(async move {
            if let Err(error) = connection.write(path, value).await {
                error!("{error:#}")
            }
        });
    }

    pub fn on_change(&self, callback: impl Fn() + Send + Sync + 'static) {
        let _guard = self.runtime.enter();
        self.connection.on_change(callback)
    }

    pub fn store_parameters(&self, path: &str, value: Value, scope: Scope) -> Result<()> {
        let connection = self.connection.clone();
        let root = self
            .repository
            .as_ref()
            .ok_or_eyre("repository not available, cannot store parameters")?
            .parameters_root();
        self.runtime.block_on(async {
            if let Err(error) = store_parameters(&connection, path, value, scope, root).await {
                error!("{error:#}")
            }
        });
        Ok(())
    }
}

async fn store_parameters(
    connection: &ConnectionHandle,
    path: &str,
    value: Value,
    scope: Scope,
    root: PathBuf,
) -> Result<()> {
    let (_, bytes) = connection.read_binary("hardware_ids").await?;
    let ids: Ids = bincode::deserialize(&bytes).wrap_err("bincode deserialization failed")?;
    let parameters = nest_value_at_path(path, value);
    parameters::directory::serialize(&parameters, scope, path, root, &ids)
        .wrap_err("serialization failed")?;
    Ok(())
}
