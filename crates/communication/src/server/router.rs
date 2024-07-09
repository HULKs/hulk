use std::{
    collections::{BTreeMap, HashMap},
    time::SystemTime,
};

use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::{
    messages::{Entry, Format, Path, RequestId, TextOrBinary},
    server::source,
};

use super::{
    connection::ConnectionHandle,
    sink::{self, SinkHandle},
    source::{SourceHandle, SubscriptionHandle},
    Tree,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("no such path: {0}")]
    NoSuchPath(Path),
    #[error("in `{source}`")]
    Source {
        source: Path,
        #[source]
        error: source::Error,
    },
    #[error("in `{sink}`")]
    Sink {
        sink: Path,
        #[source]
        error: sink::Error,
    },
}

enum Event {
    GetPaths {
        return_sender: oneshot::Sender<BTreeMap<Path, Entry>>,
    },
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
    Write {
        path: Path,
        timestamp: SystemTime,
        value: TextOrBinary,
        return_sender: oneshot::Sender<Result<(), Error>>,
    },
}

#[derive(Clone)]
pub struct RouterHandle {
    command_sender: mpsc::Sender<Event>,
}

impl RouterHandle {
    pub async fn get_paths(&self) -> BTreeMap<Path, Entry> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.command_sender
            .send(Event::GetPaths { return_sender })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub async fn read(
        &self,
        path: Path,
        format: Format,
    ) -> Result<(SystemTime, TextOrBinary), Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.command_sender
            .send(Event::Read {
                path,
                format,
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub async fn subscribe(
        &self,
        path: Path,
        format: Format,
        client: ConnectionHandle,
        id: RequestId,
    ) -> Result<(SubscriptionHandle, SystemTime, TextOrBinary), Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.command_sender
            .send(Event::Subscribe {
                path,
                format,
                client,
                id,
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }

    pub async fn write(
        &self,
        path: Path,
        timestamp: SystemTime,
        value: TextOrBinary,
    ) -> Result<(), Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.command_sender
            .send(Event::Write {
                path,
                timestamp,
                value,
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }
}

pub struct Router {
    tree: Tree,
    sources: HashMap<Path, SourceHandle>,
    sinks: HashMap<Path, SinkHandle>,
    command_receiver: mpsc::Receiver<Event>,
}

impl Router {
    pub fn new(
        tree: Tree,
        sources: HashMap<Path, SourceHandle>,
        sinks: HashMap<Path, SinkHandle>,
    ) -> (Self, RouterHandle) {
        let (command_sender, command_receiver) = mpsc::channel(1);
        let router = Self {
            tree,
            sources,
            sinks,
            command_receiver,
        };
        let handle = RouterHandle { command_sender };
        (router, handle)
    }

    pub async fn run(mut self) {
        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Event::GetPaths { return_sender } => {
                    let _ = return_sender.send(self.tree.paths.clone());
                }
                Event::Read {
                    path,
                    format,
                    return_sender,
                } => {
                    let result = self.read(path, format).await;
                    let _ = return_sender.send(result);
                }
                Event::Subscribe {
                    path,
                    format,
                    client,
                    id,
                    return_sender,
                } => {
                    let result = self.subscribe(path, format, client, id).await;
                    let _ = return_sender.send(result);
                }
                Event::Write {
                    path,
                    timestamp,
                    value,
                    return_sender,
                } => {
                    let result = self.write(path, timestamp, value).await;
                    let _ = return_sender.send(result);
                }
            }
        }
    }

    async fn read(&self, path: Path, format: Format) -> Result<(SystemTime, TextOrBinary), Error> {
        let hit = find_mount(&self.sources, &path)?;

        let response = hit
            .mount
            .read(hit.path, format)
            .await
            .map_err(|error| Error::Source {
                source: hit.mount_point.to_string(),
                error,
            })?;

        Ok(response)
    }

    async fn subscribe(
        &self,
        path: Path,
        format: Format,
        client: ConnectionHandle,
        id: RequestId,
    ) -> Result<(SubscriptionHandle, SystemTime, TextOrBinary), Error> {
        let hit = find_mount(&self.sources, &path)?;

        let response = hit
            .mount
            .subscribe(hit.path, format, client, id)
            .await
            .map_err(|error| Error::Source {
                source: hit.mount_point.to_string(),
                error,
            })?;

        Ok(response)
    }

    async fn write(
        &self,
        path: String,
        timestamp: SystemTime,
        value: TextOrBinary,
    ) -> Result<(), Error> {
        let hit = find_mount(&self.sinks, &path)?;
        hit.mount
            .write(hit.path, timestamp, value)
            .await
            .map_err(|error| Error::Sink {
                sink: hit.mount_point.to_string(),
                error,
            })?;
        Ok(())
    }
}

struct Match<'a, T> {
    mount: &'a T,
    mount_point: &'a Path,
    path: &'a str,
}

fn find_mount<'a, T>(
    mounts: impl IntoIterator<Item = (&'a Path, &'a T)>,
    path: &'a Path,
) -> Result<Match<'a, T>, Error> {
    mounts
        .into_iter()
        .find_map(|(mount_point, mount)| {
            path.strip_prefix(mount_point).map(|stripped| Match {
                mount,
                mount_point,
                path: stripped.strip_prefix(".").unwrap_or(""),
            })
        })
        .ok_or_else(|| Error::NoSuchPath(path.clone()))
}
