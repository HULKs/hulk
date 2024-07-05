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
    client::{Client, ClientHandle, PathsEvent, Status},
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
    client: ClientHandle,
    repository: Option<Repository>,
}

impl Nao {
    pub fn new(address: String) -> Self {
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

        let (client, handle) = Client::new(address);
        runtime.spawn(client.run());

        let repository = match runtime.block_on(get_repository_root()) {
            Ok(root) => Some(Repository::new(root)),
            Err(error) => {
                warn!("{error:#}");
                None
            }
        };

        Self {
            runtime,
            client: handle,
            repository,
        }
    }

    pub fn connect(&self) {
        let client = self.client.clone();
        self.runtime.spawn(async move { client.connect().await });
    }

    pub fn disconnect(&self) {
        let client = self.client.clone();
        self.runtime.spawn(async move { client.disconnect().await });
    }

    pub fn connection_status(&self) -> Status {
        self.runtime.block_on(async { self.client.status().await })
    }

    pub fn set_address(&self, address: String) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            client.set_address(address).await;
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
        let client = self.client.clone();
        spawn(async move {
            let subscription = client.subscribe_binary(path).await;
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
        let client = self.client.clone();
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
        let root = self
            .repository
            .as_ref()
            .ok_or_eyre("repository not available, cannot store parameters")?
            .parameters_root();
        self.runtime.block_on(async {
            if let Err(error) = store_parameters(&client, path, value, scope, root).await {
                error!("{error:#}")
            }
        });
        Ok(())
    }
}

async fn store_parameters(
    client: &ClientHandle,
    path: &str,
    value: Value,
    scope: Scope,
    root: PathBuf,
) -> Result<()> {
    let (_, bytes) = client.read_binary("hardware_ids").await?;
    let ids: Ids = bincode::deserialize(&bytes).wrap_err("bincode deserialization failed")?;
    let parameters = nest_value_at_path(path, value);
    parameters::directory::serialize(&parameters, scope, path, root, &ids)
        .wrap_err("serialization failed")?;
    Ok(())
}
