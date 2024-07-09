mod acceptor;
mod connection;
mod router;
mod sink;
mod source;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io,
    marker::{Send, Sync},
    time::SystemTime,
};

use log::info;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    net::{TcpListener, ToSocketAddrs},
    spawn,
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;

use crate::{
    messages::{Entry, Path},
    server::{acceptor::Acceptor, router::Router},
};

use self::{
    sink::{Sink, SinkHandle},
    source::{Source, SourceHandle},
};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RegistrationError {
    #[error("conflicting path, existing prefix: {prefix}")]
    ConflictingPath { prefix: Path },
}

#[derive(Default)]
struct Tree {
    paths: BTreeMap<Path, Entry>,
}

impl Tree {
    fn add_source(&mut self, prefix: &Path, paths: impl Iterator<Item = Path>) {
        self.paths
            .entry(prefix.to_string())
            .or_default()
            .is_readable = true;
        for path in paths {
            let concatenated_path = format!("{prefix}.{path}");
            self.paths.entry(concatenated_path).or_default().is_readable = true;
        }
    }

    fn add_sink(&mut self, prefix: &Path, paths: impl Iterator<Item = Path>) {
        self.paths
            .entry(prefix.to_string())
            .or_default()
            .is_writable = true;
        for path in paths {
            let concatenated_path = format!("{prefix}.{path}");
            self.paths.entry(concatenated_path).or_default().is_writable = true;
        }
    }
}

#[derive(Default)]
pub struct Server {
    tree: Tree,
    sources: HashMap<Path, SourceHandle>,
    sinks: HashMap<Path, SinkHandle>,
    tasks: JoinSet<()>,
}

impl Server {
    pub async fn serve(
        mut self,
        addresses: impl ToSocketAddrs + Send,
        cancellation_token: CancellationToken,
    ) -> Result<(), io::Error> {
        let listener = TcpListener::bind(addresses).await?;
        let (router, router_handle) = Router::new(self.tree, self.sources, self.sinks);
        let router_task = spawn(router.run());

        Acceptor::new(listener, router_handle, cancellation_token.clone())
            .run()
            .await;

        router_task.await.unwrap();
        while let Some(result) = self.tasks.join_next().await {
            result.unwrap();
        }
        info!("server stopped");
        Ok(())
    }

    pub fn expose_source<T>(
        &mut self,
        path: impl Into<String>,
        data: buffered_watch::Receiver<(SystemTime, T)>,
        subscriptions: buffered_watch::Sender<HashSet<Path>>,
    ) -> Result<(), RegistrationError>
    where
        T: Serialize + PathSerialize + PathIntrospect + Send + Sync + 'static,
    {
        let path = path.into();
        if let Some(prefix) = self.sources.keys().find(|&key| path.starts_with(key)) {
            return Err(RegistrationError::ConflictingPath {
                prefix: prefix.clone(),
            });
        }
        self.tree.add_source(&path, T::get_fields().into_iter());
        let (source, handle) = Source::new(data, subscriptions);
        self.sources.insert(path, handle);
        self.tasks.spawn(source.run());
        Ok(())
    }

    pub fn expose_sink<T>(
        &mut self,
        path: impl Into<String>,
        data: buffered_watch::Sender<(SystemTime, T)>,
    ) -> Result<(), RegistrationError>
    where
        T: Clone + PathDeserialize + PathIntrospect + Send + Sync + 'static,
        for<'de> T: Deserialize<'de>,
    {
        let path = path.into();
        if let Some(prefix) = self.sinks.keys().find(|&key| path.starts_with(key)) {
            return Err(RegistrationError::ConflictingPath {
                prefix: prefix.clone(),
            });
        }
        self.tree.add_sink(&path, T::get_fields().into_iter());
        let (sink, handle) = Sink::new(data);
        self.sinks.insert(path, handle);
        self.tasks.spawn(sink.run());
        Ok(())
    }
}
