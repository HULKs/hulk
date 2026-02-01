//! Session management for Zenoh connections.
//!
//! A [`Session`] is the entry point for all hulkz operations. It manages the
//! underlying Zenoh connection and provides the namespace context for nodes.
//!
//! # Example
//!
//! ```rust,no_run
//! # use hulkz::{Session, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let session = Session::create("robot").await?;
//! let node = session.create_node("navigation").build().await?;
//!
//! // Discovery
//! let nodes = session.list_nodes().await?;
//! let (watcher, driver) = session.watch_nodes().await?;
//! tokio::spawn(driver);
//! # Ok(())
//! # }
//! ```

use std::{future::Future, sync::Arc};

use tokio::sync::mpsc;
use zenoh::liveliness::LivelinessToken;

use crate::{
    config::Config,
    error::Result,
    graph::{
        parse_node_key, parse_session_key, NodeEvent, NodeWatcher, ParameterInfo, PublisherEvent,
        PublisherInfo, PublisherWatcher, SessionEvent, SessionWatcher,
    },
    key::{
        graph_nodes_pattern, graph_publishers_pattern, graph_session_key, graph_sessions_pattern,
        param_read_global_pattern, param_read_pattern, param_read_private_pattern,
    },
    node::NodeBuilder,
    Timestamp,
};

/// Builder for creating a [`Session`].
pub struct SessionBuilder {
    namespace: String,
    config: Config,
    config_files: Vec<String>,
}

impl SessionBuilder {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            config: Config::new(),
            config_files: Vec::new(),
        }
    }

    /// Adds a parameter configuration file to load.
    ///
    /// Files are loaded in order, with later files overriding earlier values.
    /// This is called after loading defaults from environment/convention.
    pub fn parameters_file(mut self, path: impl Into<String>) -> Self {
        self.config_files.push(path.into());
        self
    }

    pub async fn build(mut self) -> Result<Session> {
        tracing::info!("Opening new Zenoh session");

        // Load config: environment/convention first, then explicit files
        self.config = Config::load_default().await?;
        for path in &self.config_files {
            self.config.load_file(path).await?;
        }

        let zenoh_config = if std::env::var(zenoh::Config::DEFAULT_CONFIG_PATH_ENV).is_ok() {
            zenoh::Config::from_env()?
        } else {
            zenoh::Config::default()
        };

        let session = zenoh::open(zenoh_config).await?;

        // Generate session ID: {uuid}@{hostname}
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .into_owned();
        let session_id = format!("{}@{}", uuid::Uuid::new_v4(), hostname);

        // Declare session liveliness token for discovery
        let liveliness_key = graph_session_key(&self.namespace, &session_id);
        let liveliness_token = session
            .liveliness()
            .declare_token(&liveliness_key)
            .await?;

        let inner = SessionInner {
            zenoh: session,
            namespace: self.namespace,
            session_id,
            config: self.config,
            _liveliness_token: liveliness_token,
        };
        Ok(Session {
            inner: Arc::new(inner),
        })
    }
}

/// A Zenoh session scoped to a robot namespace.
///
/// The session is the entry point for all Hulkz operations. It manages the
/// underlying Zenoh connection and provides the namespace context for nodes.
#[derive(Clone, Debug)]
pub struct Session {
    inner: Arc<SessionInner>,
}

#[derive(Debug)]
struct SessionInner {
    zenoh: zenoh::Session,
    namespace: String,
    session_id: String,
    config: Config,
    _liveliness_token: LivelinessToken,
}

impl Session {
    /// Creates a new session with the given namespace.
    ///
    /// This is a convenience method that uses default configuration.
    /// For more control, use [`Session::builder`].
    pub async fn create(namespace: impl Into<String>) -> Result<Self> {
        Self::builder(namespace).build().await
    }

    /// Creates a session builder for more configuration options.
    pub fn builder(namespace: impl Into<String>) -> SessionBuilder {
        SessionBuilder::new(namespace)
    }

    /// Creates a node within this session.
    pub fn create_node(&self, name: impl Into<String>) -> NodeBuilder {
        NodeBuilder {
            session: self.clone(),
            name: name.into(),
        }
    }

    /// Returns the current Zenoh timestamp.
    pub fn now(&self) -> Timestamp {
        self.inner.zenoh.new_timestamp()
    }

    pub(crate) fn zenoh(&self) -> &zenoh::Session {
        &self.inner.zenoh
    }

    pub(crate) fn config(&self) -> &Config {
        &self.inner.config
    }

    pub fn namespace(&self) -> &str {
        &self.inner.namespace
    }

    /// Returns the unique session ID.
    ///
    /// Format: `{uuid}@{hostname}`
    pub fn id(&self) -> &str {
        &self.inner.session_id
    }

    // =========================================================================
    // Discovery API - List methods
    // =========================================================================

    /// Lists all sessions in the current namespace.
    pub async fn list_sessions(&self) -> Result<Vec<String>> {
        self.list_sessions_in_namespace(&self.inner.namespace).await
    }

    /// Lists all sessions in the given namespace.
    pub async fn list_sessions_in_namespace(&self, namespace: &str) -> Result<Vec<String>> {
        let pattern = graph_sessions_pattern(namespace);
        let replies = self.inner.zenoh.liveliness().get(&pattern).await?;
        let mut sessions = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.into_result() {
                if let Some(session_id) = parse_session_key(sample.key_expr().as_str()) {
                    sessions.push(session_id);
                }
            }
        }

        Ok(sessions)
    }

    /// Lists all nodes in the current namespace.
    pub async fn list_nodes(&self) -> Result<Vec<String>> {
        self.list_nodes_in_namespace(&self.inner.namespace).await
    }

    /// Lists all nodes in the given namespace.
    pub async fn list_nodes_in_namespace(&self, namespace: &str) -> Result<Vec<String>> {
        let pattern = graph_nodes_pattern(namespace);
        let replies = self.inner.zenoh.liveliness().get(&pattern).await?;
        let mut nodes = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.into_result() {
                if let Some(node_name) = parse_node_key(sample.key_expr().as_str()) {
                    nodes.push(node_name);
                }
            }
        }

        Ok(nodes)
    }

    /// Lists all publishers in the current namespace.
    pub async fn list_publishers(&self) -> Result<Vec<PublisherInfo>> {
        self.list_publishers_in_namespace(&self.inner.namespace)
            .await
    }

    /// Lists all publishers in the given namespace.
    pub async fn list_publishers_in_namespace(&self, namespace: &str) -> Result<Vec<PublisherInfo>> {
        let pattern = graph_publishers_pattern(namespace);
        let replies = self.inner.zenoh.liveliness().get(&pattern).await?;
        let mut publishers = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.into_result() {
                if let Some(info) = PublisherInfo::from_key(sample.key_expr().as_str()) {
                    publishers.push(info);
                }
            }
        }

        Ok(publishers)
    }

    /// Lists all parameters in the current namespace.
    ///
    /// This discovers parameters by querying the param read plane.
    /// Returns parameters from all scopes (global, local, private).
    pub async fn list_parameters(&self) -> Result<Vec<ParameterInfo>> {
        self.list_parameters_in_namespace(&self.inner.namespace)
            .await
    }

    /// Lists all parameters in the given namespace.
    ///
    /// This discovers parameters by querying the param read plane.
    /// Returns parameters from all scopes (global, local, private).
    pub async fn list_parameters_in_namespace(&self, namespace: &str) -> Result<Vec<ParameterInfo>> {
        let mut parameters = Vec::new();

        // Query global parameters
        let global_pattern = param_read_global_pattern();
        self.collect_parameters(&global_pattern, &mut parameters)
            .await?;

        // Query local parameters for this namespace
        let local_pattern = param_read_pattern(namespace);
        self.collect_parameters(&local_pattern, &mut parameters)
            .await?;

        // Query private parameters for this namespace
        let private_pattern = param_read_private_pattern(namespace);
        self.collect_parameters(&private_pattern, &mut parameters)
            .await?;

        Ok(parameters)
    }

    /// Helper to collect parameters from a query pattern.
    async fn collect_parameters(
        &self,
        pattern: &str,
        parameters: &mut Vec<ParameterInfo>,
    ) -> Result<()> {
        let replies = self
            .inner
            .zenoh
            .get(pattern)
            .timeout(std::time::Duration::from_millis(500))
            .await?;

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.into_result() {
                if let Some(info) = ParameterInfo::from_key(sample.key_expr().as_str()) {
                    parameters.push(info);
                }
            }
        }

        Ok(())
    }

    // =========================================================================
    // Discovery API - Watch methods
    // =========================================================================

    /// Watches for node join/leave events in the current namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_nodes(&self) -> Result<(NodeWatcher, impl Future<Output = ()> + Send)> {
        self.watch_nodes_in_namespace(&self.inner.namespace).await
    }

    /// Watches for node join/leave events in the given namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_nodes_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<(NodeWatcher, impl Future<Output = ()> + Send)> {
        let pattern = graph_nodes_pattern(namespace);
        let subscriber = self
            .inner
            .zenoh
            .liveliness()
            .declare_subscriber(&pattern)
            .await?;

        let (tx, rx) = mpsc::channel(32);
        let watcher = NodeWatcher::new(rx);

        let driver = async move {
            loop {
                match subscriber.recv_async().await {
                    Ok(sample) => {
                        let key = sample.key_expr().as_str();
                        if let Some(node_name) = parse_node_key(key) {
                            let event = if sample.kind() == zenoh::sample::SampleKind::Put {
                                NodeEvent::Joined(node_name)
                            } else {
                                NodeEvent::Left(node_name)
                            };
                            if tx.send(event).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        Ok((watcher, driver))
    }

    /// Watches for session join/leave events in the current namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_sessions(&self) -> Result<(SessionWatcher, impl Future<Output = ()> + Send)> {
        self.watch_sessions_in_namespace(&self.inner.namespace).await
    }

    /// Watches for session join/leave events in the given namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_sessions_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<(SessionWatcher, impl Future<Output = ()> + Send)> {
        let pattern = graph_sessions_pattern(namespace);
        let subscriber = self
            .inner
            .zenoh
            .liveliness()
            .declare_subscriber(&pattern)
            .await?;

        let (tx, rx) = mpsc::channel(32);
        let watcher = SessionWatcher::new(rx);

        let driver = async move {
            loop {
                match subscriber.recv_async().await {
                    Ok(sample) => {
                        let key = sample.key_expr().as_str();
                        if let Some(session_id) = parse_session_key(key) {
                            let event = if sample.kind() == zenoh::sample::SampleKind::Put {
                                SessionEvent::Joined(session_id)
                            } else {
                                SessionEvent::Left(session_id)
                            };
                            if tx.send(event).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        Ok((watcher, driver))
    }

    /// Watches for publisher advertise/unadvertise events in the current namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_publishers(&self) -> Result<(PublisherWatcher, impl Future<Output = ()> + Send)> {
        self.watch_publishers_in_namespace(&self.inner.namespace)
            .await
    }

    /// Watches for publisher advertise/unadvertise events in the given namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_publishers_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<(PublisherWatcher, impl Future<Output = ()> + Send)> {
        let pattern = graph_publishers_pattern(namespace);
        let subscriber = self
            .inner
            .zenoh
            .liveliness()
            .declare_subscriber(&pattern)
            .await?;

        let (tx, rx) = mpsc::channel(32);
        let watcher = PublisherWatcher::new(rx);

        let driver = async move {
            loop {
                match subscriber.recv_async().await {
                    Ok(sample) => {
                        let key = sample.key_expr().as_str();
                        if let Some(info) = PublisherInfo::from_key(key) {
                            let event = if sample.kind() == zenoh::sample::SampleKind::Put {
                                PublisherEvent::Advertised(info)
                            } else {
                                PublisherEvent::Unadvertised(info)
                            };
                            if tx.send(event).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        Ok((watcher, driver))
    }

    // =========================================================================
    // Parameter API - Query and set parameters remotely
    // =========================================================================

    /// Queries a parameter value by key expression.
    ///
    /// Returns the JSON value if found, or None if not found.
    pub async fn query_parameter(&self, key_expr: &str) -> Result<Option<serde_json::Value>> {
        let replies = self.inner.zenoh.get(key_expr).await?;

        while let Ok(reply) = replies.recv_async().await {
            match reply.into_result() {
                Ok(sample) => {
                    let value: serde_json::Value =
                        serde_json::from_slice(&sample.payload().to_bytes())
                            .map_err(crate::Error::JsonDeserialize)?;
                    return Ok(Some(value));
                }
                Err(_) => continue,
            }
        }

        Ok(None)
    }

    /// Sets a parameter value by key expression.
    ///
    /// Returns `Ok(None)` if successful, `Ok(Some(error_message))` if the parameter
    /// rejected the value (e.g., validation failed), or `Err` if no responder was found.
    pub async fn set_parameter(
        &self,
        key_expr: &str,
        value: &serde_json::Value,
    ) -> Result<Option<String>> {
        let payload =
            serde_json::to_vec(value).map_err(crate::Error::JsonSerialize)?;

        let replies = self
            .inner
            .zenoh
            .get(key_expr)
            .payload(payload)
            .encoding(zenoh::bytes::Encoding::APPLICATION_JSON)
            .await?;

        while let Ok(reply) = replies.recv_async().await {
            match reply.into_result() {
                Ok(_sample) => {
                    // Success
                    return Ok(None);
                }
                Err(err_reply) => {
                    // Parameter rejected the value (e.g., validation failed)
                    let error_msg = String::from_utf8_lossy(&err_reply.payload().to_bytes())
                        .to_string();
                    return Ok(Some(error_msg));
                }
            }
        }

        // No replies received
        Err(crate::Error::ParameterNotFound(key_expr.to_string()))
    }
}
