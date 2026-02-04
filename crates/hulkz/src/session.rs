//! Session management for Zenoh connections.
//!
//! A [`Session`] is the entry point for all hulkz operations. It manages the underlying Zenoh
//! connection and provides the namespace context for nodes.
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
use zenoh::{handlers::FifoChannelHandler, liveliness::LivelinessToken, sample::Sample};

use crate::{
    config::Config,
    error::{Error, Result},
    graph::{
        parse_node_key, parse_session_key, NodeEvent, NodeInfo, NodeWatcher, ParameterEvent,
        ParameterInfo, ParameterWatcher, PublisherEvent, PublisherInfo, PublisherWatcher,
        SessionEvent, SessionInfo, SessionWatcher,
    },
    key::{
        graph_all_nodes_pattern, graph_all_parameters_pattern, graph_all_publishers_pattern,
        graph_all_sessions_pattern, graph_nodes_pattern, graph_parameters_pattern,
        graph_publishers_pattern, graph_session_key, graph_sessions_pattern, ParamIntent, Scope,
    },
    node::NodeBuilder,
    scoped_path::ScopedPath,
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
    /// Files are loaded in order, with later files overriding earlier values. This is called after
    /// loading defaults from environment/convention.
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
        let unique_id = uuid::Uuid::new_v4();
        let hostname = gethostname::gethostname().to_string_lossy().into_owned();
        let session_id = format!("{unique_id}@{hostname}");

        // Declare session liveliness token for discovery
        let liveliness_key = graph_session_key(&self.namespace, &session_id);
        let liveliness_token = session.liveliness().declare_token(&liveliness_key).await?;

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
/// The session is the entry point for all hulkz operations. It manages the underlying Zenoh
/// connection and provides the namespace context for nodes.
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
    /// This is a convenience method that uses default configuration. For more control, use
    /// [`Session::builder`].
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
    pub fn id(&self) -> &str {
        &self.inner.session_id
    }

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
            if let Ok(sample) = reply.result() {
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
            if let Ok(sample) = reply.result() {
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
    pub async fn list_publishers_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<Vec<PublisherInfo>> {
        let pattern = graph_publishers_pattern(namespace);
        let replies = self.inner.zenoh.liveliness().get(&pattern).await?;
        let mut publishers = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                if let Some(info) = PublisherInfo::from_key(sample.key_expr().as_str()) {
                    publishers.push(info);
                }
            }
        }

        Ok(publishers)
    }

    /// Lists all parameters in the current namespace.
    ///
    /// This discovers parameters via liveliness tokens on the graph plane. Returns parameters from
    /// all scopes (global, local, private).
    pub async fn list_parameters(&self) -> Result<Vec<ParameterInfo>> {
        self.list_parameters_in_namespace(&self.inner.namespace)
            .await
    }

    /// Lists all parameters in the given namespace.
    ///
    /// This discovers parameters via liveliness tokens on the graph plane. Returns parameters from
    /// all scopes (global, local, private).
    pub async fn list_parameters_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<Vec<ParameterInfo>> {
        let pattern = graph_parameters_pattern(namespace);
        let replies = self.inner.zenoh.liveliness().get(&pattern).await?;
        let mut parameters = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                if let Some(info) = ParameterInfo::from_key(sample.key_expr().as_str()) {
                    parameters.push(info);
                }
            }
        }

        Ok(parameters)
    }

    /// Watches for node join/leave events in the current namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_nodes(
        &self,
    ) -> Result<(NodeWatcher, impl Future<Output = Result<()>> + Send)> {
        self.watch_nodes_in_namespace(&self.inner.namespace).await
    }

    /// Watches for node join/leave events in the given namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_nodes_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<(NodeWatcher, impl Future<Output = Result<()>> + Send)> {
        let pattern = graph_nodes_pattern(namespace);
        let subscriber = self
            .inner
            .zenoh
            .liveliness()
            .declare_subscriber(&pattern)
            .await?;

        let (tx, rx) = mpsc::channel(32);
        let watcher = NodeWatcher::new(rx);

        let driver = drive_node_watcher(subscriber, tx);

        Ok((watcher, driver))
    }

    /// Watches for session join/leave events in the current namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_sessions(
        &self,
    ) -> Result<(SessionWatcher, impl Future<Output = Result<()>> + Send)> {
        self.watch_sessions_in_namespace(&self.inner.namespace)
            .await
    }

    /// Watches for session join/leave events in the given namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_sessions_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<(SessionWatcher, impl Future<Output = Result<()>> + Send)> {
        let pattern = graph_sessions_pattern(namespace);
        let subscriber = self
            .inner
            .zenoh
            .liveliness()
            .declare_subscriber(&pattern)
            .await?;

        let (tx, rx) = mpsc::channel(32);
        let watcher = SessionWatcher::new(rx);

        let driver = drive_session_watcher(subscriber, tx);

        Ok((watcher, driver))
    }

    /// Watches for publisher advertise/unadvertise events in the current namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_publishers(
        &self,
    ) -> Result<(PublisherWatcher, impl Future<Output = Result<()>> + Send)> {
        self.watch_publishers_in_namespace(&self.inner.namespace)
            .await
    }

    /// Watches for publisher advertise/unadvertise events in the given namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_publishers_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<(PublisherWatcher, impl Future<Output = Result<()>> + Send)> {
        let pattern = graph_publishers_pattern(namespace);
        let subscriber = self
            .inner
            .zenoh
            .liveliness()
            .declare_subscriber(&pattern)
            .await?;

        let (tx, rx) = mpsc::channel(32);
        let watcher = PublisherWatcher::new(rx);

        let driver = drive_publisher_watcher(subscriber, tx);

        Ok((watcher, driver))
    }

    /// Watches for parameter declare/undeclare events in the current namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_parameters(
        &self,
    ) -> Result<(ParameterWatcher, impl Future<Output = Result<()>> + Send)> {
        self.watch_parameters_in_namespace(&self.inner.namespace)
            .await
    }

    /// Watches for parameter declare/undeclare events in the given namespace.
    ///
    /// Returns a watcher and a driver future that must be spawned.
    pub async fn watch_parameters_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<(ParameterWatcher, impl Future<Output = Result<()>> + Send)> {
        let pattern = graph_parameters_pattern(namespace);
        let subscriber = self
            .inner
            .zenoh
            .liveliness()
            .declare_subscriber(&pattern)
            .await?;

        let (tx, rx) = mpsc::channel(32);
        let watcher = ParameterWatcher::new(rx);

        let driver = drive_parameter_watcher(subscriber, tx);

        Ok((watcher, driver))
    }

    /// Access a parameter by path for reading or writing.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use hulkz::{Session, Result};
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let session = Session::create("robot").await?;
    ///
    /// // Get local parameter
    /// let value = session.parameter("max_speed").get().await?;
    ///
    /// // Set global parameter
    /// session.parameter("/fleet_id").set(&serde_json::json!("fleet-01")).await?;
    ///
    /// // Get private parameter (requires node)
    /// let debug = session.parameter("~/debug").on_node("motor").get().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn parameter(&self, path: &str) -> ParamAccessBuilder<'_> {
        ParamAccessBuilder {
            session: self,
            path: ScopedPath::parse(path),
            node: None,
            namespace_override: None,
        }
    }

    /// Queries a parameter value by raw key expression.
    ///
    /// Returns the JSON value if found, or None if not found.
    pub(crate) async fn query_parameter_raw(
        &self,
        key_expr: &str,
    ) -> Result<Option<serde_json::Value>> {
        let replies = self.inner.zenoh.get(key_expr).await?;

        while let Ok(reply) = replies.recv_async().await {
            match reply.result() {
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

    /// Sets a parameter value by raw key expression.
    pub(crate) async fn set_parameter_raw(
        &self,
        key_expr: &str,
        value: &serde_json::Value,
    ) -> Result<()> {
        let payload = serde_json::to_vec(value).map_err(crate::Error::JsonSerialize)?;

        let replies = self
            .inner
            .zenoh
            .get(key_expr)
            .payload(payload)
            .encoding(zenoh::bytes::Encoding::APPLICATION_JSON)
            .await?;

        let mut success_count = 0;
        let mut rejections = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            match reply.result() {
                Ok(_sample) => success_count += 1,
                Err(err_reply) => {
                    let reason =
                        String::from_utf8_lossy(&err_reply.payload().to_bytes()).to_string();
                    rejections.push(reason);
                }
            }
        }

        if !rejections.is_empty() {
            return Err(crate::Error::ParameterRejected(rejections));
        }

        if success_count == 0 {
            return Err(crate::Error::ParameterNotFound(key_expr.to_string()));
        }

        Ok(())
    }

    /// Lists all sessions across all namespaces.
    ///
    /// This discovers sessions globally, not limited to any particular namespace.
    /// Useful for debug tools that need to see the entire network topology.
    pub async fn list_all_sessions(&self) -> Result<Vec<SessionInfo>> {
        let pattern = graph_all_sessions_pattern();
        let replies = self.inner.zenoh.liveliness().get(&pattern).await?;
        let mut sessions = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                if let Some(info) = SessionInfo::from_key(sample.key_expr().as_str()) {
                    sessions.push(info);
                }
            }
        }

        Ok(sessions)
    }

    /// Lists all nodes across all namespaces.
    ///
    /// This discovers nodes globally, not limited to any particular namespace. Useful for debug
    /// tools that need to see the entire network topology.
    pub async fn list_all_nodes(&self) -> Result<Vec<NodeInfo>> {
        let pattern = graph_all_nodes_pattern();
        let replies = self.inner.zenoh.liveliness().get(&pattern).await?;
        let mut nodes = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                if let Some(info) = NodeInfo::from_key(sample.key_expr().as_str()) {
                    nodes.push(info);
                }
            }
        }

        Ok(nodes)
    }

    /// Lists all publishers across all namespaces.
    ///
    /// This discovers publishers globally, not limited to any particular namespace. Useful for
    /// debug tools that need to see the entire network topology.
    pub async fn list_all_publishers(&self) -> Result<Vec<PublisherInfo>> {
        let pattern = graph_all_publishers_pattern();
        let replies = self.inner.zenoh.liveliness().get(&pattern).await?;
        let mut publishers = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                if let Some(info) = PublisherInfo::from_key(sample.key_expr().as_str()) {
                    publishers.push(info);
                }
            }
        }

        Ok(publishers)
    }

    /// Lists all parameters across all namespaces.
    ///
    /// This discovers parameters globally, not limited to any particular namespace. Useful for
    /// debug tools that need to see the entire network topology.
    pub async fn list_all_parameters(&self) -> Result<Vec<ParameterInfo>> {
        let pattern = graph_all_parameters_pattern();
        let replies = self.inner.zenoh.liveliness().get(&pattern).await?;
        let mut parameters = Vec::new();

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                if let Some(info) = ParameterInfo::from_key(sample.key_expr().as_str()) {
                    parameters.push(info);
                }
            }
        }

        Ok(parameters)
    }

    /// Lists all discovered namespaces.
    ///
    /// This is a convenience method that discovers all sessions and extracts unique namespaces
    /// from them.
    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        let sessions = self.list_all_sessions().await?;
        let mut namespaces: Vec<String> = sessions
            .into_iter()
            .map(|s| s.namespace)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        namespaces.sort();
        Ok(namespaces)
    }
}

/// Builder for parameter access operations.
///
/// Created via [`Session::parameter()`]. Use `.on_node()` to target a specific node (required for
/// private parameters).
///
/// # Example
///
/// ```rust,no_run
/// # use hulkz::{Session, Result};
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// let session = Session::create("robot").await?;
///
/// // Local parameter (default scope)
/// let value = session.parameter("max_speed").get().await?;
///
/// // Global parameter
/// session.parameter("/fleet_id").set(&serde_json::json!("fleet-01")).await?;
///
/// // Private parameter on specific node
/// let debug = session.parameter("~/debug").on_node("motor").get().await?;
/// # Ok(())
/// # }
/// ```
pub struct ParamAccessBuilder<'a> {
    session: &'a Session,
    path: ScopedPath,
    node: Option<String>,
    /// Optional namespace override for cross-namespace access.
    namespace_override: Option<String>,
}

impl<'a> ParamAccessBuilder<'a> {
    /// Target a specific node.
    ///
    /// Required for private parameters (`~/path`).
    pub fn on_node(mut self, node: &str) -> Self {
        self.node = Some(node.to_string());
        self
    }

    /// Override the namespace for this parameter access.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use hulkz::{Session, Result};
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let session = Session::create("twix").await?;
    ///
    /// // Read a parameter from a different namespace
    /// let value = session.parameter("max_speed")
    ///     .in_namespace("robot-nao22")
    ///     .on_node("control")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn in_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace_override = Some(namespace.into());
        self
    }

    /// Get the parameter value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NodeRequiredForPrivate`] if this is a private parameter and `.on_node()`
    /// was not called.
    pub async fn get(self) -> Result<Option<serde_json::Value>> {
        let node_name = self.resolve_node()?;
        let namespace = self.resolve_namespace();
        let read_key = self
            .path
            .to_param_key(ParamIntent::Read, &namespace, &node_name);
        self.session.query_parameter_raw(&read_key).await
    }

    /// Set the parameter value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NodeRequiredForPrivate`] if this is a private parameter and `.on_node()`
    /// was not called.
    ///
    /// Returns [`Error::ParameterNotFound`] if no node is serving this parameter.
    ///
    /// Returns [`Error::ParameterRejected`] if the parameter validation failed.
    pub async fn set(self, value: &serde_json::Value) -> Result<()> {
        let node_name = self.resolve_node()?;
        let namespace = self.resolve_namespace();
        let write_key = self
            .path
            .to_param_key(ParamIntent::Write, &namespace, &node_name);
        self.session.set_parameter_raw(&write_key, value).await
    }

    /// Resolves the namespace to use.
    fn resolve_namespace(&self) -> String {
        self.namespace_override
            .clone()
            .unwrap_or_else(|| self.session.namespace().to_string())
    }

    /// Resolves the node name for the key expression.
    ///
    /// - For private scope: requires explicit node, returns error if not set
    /// - For global/local scope: uses explicit node if set, otherwise wildcard
    fn resolve_node(&self) -> Result<String> {
        match (self.path.scope(), &self.node) {
            (Scope::Private, None) => Err(Error::NodeRequiredForPrivate),
            (_, Some(node)) => Ok(node.clone()),
            (_, None) => Ok("*".to_string()),
        }
    }
}

async fn drive_node_watcher(
    subscriber: zenoh::pubsub::Subscriber<FifoChannelHandler<Sample>>,
    tx: mpsc::Sender<NodeEvent>,
) -> Result<()> {
    loop {
        let sample = subscriber.recv_async().await?;
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
    Ok(())
}

async fn drive_session_watcher(
    subscriber: zenoh::pubsub::Subscriber<FifoChannelHandler<Sample>>,
    tx: mpsc::Sender<SessionEvent>,
) -> Result<()> {
    loop {
        let sample = subscriber.recv_async().await?;
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
    Ok(())
}

async fn drive_publisher_watcher(
    subscriber: zenoh::pubsub::Subscriber<FifoChannelHandler<Sample>>,
    tx: mpsc::Sender<PublisherEvent>,
) -> Result<()> {
    loop {
        let sample = subscriber.recv_async().await?;
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
    Ok(())
}

async fn drive_parameter_watcher(
    subscriber: zenoh::pubsub::Subscriber<FifoChannelHandler<Sample>>,
    tx: mpsc::Sender<ParameterEvent>,
) -> Result<()> {
    loop {
        let sample = subscriber.recv_async().await?;
        let key = sample.key_expr().as_str();
        if let Some(info) = ParameterInfo::from_key(key) {
            let event = if sample.kind() == zenoh::sample::SampleKind::Put {
                ParameterEvent::Declared(info)
            } else {
                ParameterEvent::Undeclared(info)
            };
            if tx.send(event).await.is_err() {
                break;
            }
        }
    }
    Ok(())
}
