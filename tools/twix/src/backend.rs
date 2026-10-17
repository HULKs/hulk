use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use color_eyre::{Result, eyre::Context as _};
use ros_z::{context::ContextBuilder, graph::Graph, node::Node, prelude::*};
use ros_z_debug::query::{Connection, Cursor, CursorHandle, Live, LiveRetention, TopicList};
use ros_z_debug::{TopicObserver, TopicObserverOptions};
use tokio::runtime::Handle;
use uuid::Uuid;

pub struct RobotBackend {
    runtime_handle: Handle,
    context: Arc<Context>,
    node: Arc<Node>,
    observer: TopicObserver,
    namespace: Mutex<String>,
    connection_generation: AtomicU64,
    connection: Mutex<Connection>,
    live: Arc<Live>,
}

impl RobotBackend {
    pub async fn new(
        runtime_handle: Handle,
        router: Option<String>,
        namespace: String,
    ) -> Result<Self> {
        let mut builder = ContextBuilder::default();
        if let Some(router) = router {
            builder = builder
                .with_router_endpoint(router)
                .wrap_err("failed to configure ROS-Z router endpoint")?;
        }

        let context = Arc::new(
            builder
                .build()
                .await
                .wrap_err("failed to build ROS-Z context")?,
        );
        let node_name = twix_node_name();
        let node = Arc::new(
            context
                .create_node(node_name)
                .with_namespace("/_twix")
                .build()
                .await
                .wrap_err("failed to create Twix ROS-Z node")?,
        );
        let options = TopicObserverOptions::with_namespace(namespace.clone())
            .wrap_err("failed to configure initial Twix namespace")?;
        let observer = TopicObserver::new(Arc::clone(&node), options);
        let live = Arc::new(Live::new(Arc::clone(&node), LiveRetention::default()));
        let connection = Connection {
            backend: live.clone(),
            cursor: CursorHandle::new(Cursor::Live),
        };
        observer.set_backend(connection.clone());

        Ok(Self {
            runtime_handle,
            context,
            node,
            observer,
            namespace: Mutex::new(namespace),
            connection_generation: AtomicU64::new(0),
            connection: Mutex::new(connection),
            live,
        })
    }

    pub fn runtime_handle(&self) -> &Handle {
        &self.runtime_handle
    }

    pub fn node(&self) -> Arc<Node> {
        self.node.clone()
    }

    pub fn graph(&self) -> &Graph {
        self.context.graph().as_ref()
    }

    pub fn observer(&self) -> &TopicObserver {
        &self.observer
    }

    pub fn query_connection(&self) -> Connection {
        self.connection.lock().expect("connection mutex").clone()
    }

    pub fn topics(&self) -> TopicList {
        self.connection
            .lock()
            .expect("connection mutex")
            .backend
            .topics()
    }

    pub fn use_live(&self) {
        self.set_connection(Connection {
            backend: self.live.clone(),
            cursor: CursorHandle::new(Cursor::Live),
        });
    }

    fn set_connection(&self, connection: Connection) {
        self.observer.set_backend(connection.clone());
        let mut current = self.connection.lock().expect("connection mutex");
        *current = connection;
        self.connection_generation.fetch_add(1, Ordering::Relaxed);
    }

    pub fn namespace(&self) -> String {
        self.namespace
            .lock()
            .expect("namespace mutex should not be poisoned")
            .clone()
    }

    pub fn set_namespace(&self, namespace: String) -> Result<()> {
        self.observer
            .set_namespace(namespace.clone())
            .wrap_err("failed to set Twix target namespace")?;
        *self
            .namespace
            .lock()
            .expect("namespace mutex should not be poisoned") = namespace;
        Ok(())
    }

    /// Connection replacements and cursor seeks independently invalidate presentation state.
    pub fn presentation_identity(&self) -> (u64, u64) {
        let connection = self.connection.lock().expect("connection mutex");
        (
            self.connection_generation.load(Ordering::Relaxed),
            connection.cursor.position().revision,
        )
    }
}

impl Drop for RobotBackend {
    fn drop(&mut self) {
        if let Err(error) = self.context.shutdown() {
            log::error!("failed to shut down ROS-Z context: {error:#}");
        }
    }
}

fn twix_node_name() -> String {
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown-host".to_string());
    let host = sanitize_node_component(&host);
    let id = Uuid::new_v4().simple().to_string();
    let short_id = &id[..8];
    format!("twix_{short_id}_{host}")
}

fn sanitize_node_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "unknown-host".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn connection_replacements_have_distinct_presentation_identities_at_revision_zero() {
        let backend = RobotBackend::new(Handle::current(), None, "/".into())
            .await
            .unwrap();
        let initial = backend.presentation_identity();
        assert_eq!(initial.1, 0);

        backend.set_connection(Connection {
            backend: Arc::new(Live::new(backend.node(), LiveRetention::default())),
            cursor: CursorHandle::new(Cursor::Live),
        });
        let replacement = backend.presentation_identity();
        assert_eq!(replacement.1, 0);
        assert_ne!(replacement.0, initial.0);

        backend.use_live();
        let live = backend.presentation_identity();
        assert_eq!(live.1, 0);
        assert_ne!(live.0, replacement.0);
        assert_ne!(live.0, initial.0);
    }
}
