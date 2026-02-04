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

use std::{path::PathBuf, sync::Arc};

use zenoh::liveliness::LivelinessToken;

use crate::{
    config::Config,
    error::{Error, Result},
    graph::GraphAccess,
    key::{GraphKey, ParamIntent, ParamKey},
    node::NodeBuilder,
    scoped_path::ScopedPath,
    Scope, Timestamp,
};

/// Builder for creating a [`Session`].
pub struct SessionBuilder {
    namespace: String,
    config: Config,
    config_files: Vec<PathBuf>,
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
    pub fn overlay_parameters_file(mut self, path: impl Into<PathBuf>) -> Self {
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
        let liveliness_key = GraphKey::session(&self.namespace, &session_id);
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

    /// Access the graph plane for discovery operations.
    ///
    /// Returns a builder for listing and watching sessions, nodes, publishers, and parameters.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use hulkz::{Session, Result, GraphEvent};
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let session = Session::create("robot").await?;
    ///
    /// // List nodes in current namespace
    /// let nodes = session.graph().nodes().list().await?;
    ///
    /// // List publishers in a specific namespace
    /// let pubs = session.graph().in_namespace("other").publishers().list().await?;
    ///
    /// // Watch all nodes across all namespaces
    /// let (mut watcher, driver) = session.graph().all_namespaces().nodes().watch().await?;
    /// tokio::spawn(driver);
    /// while let Some(event) = watcher.recv().await {
    ///     match event {
    ///         GraphEvent::Joined(info) => println!("+ {}", info.node),
    ///         GraphEvent::Left(info) => println!("- {}", info.node),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn graph(&self) -> GraphAccess<'_> {
        GraphAccess::new(self)
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
        let read_key = ParamKey::from_scope(
            ParamIntent::Read,
            self.path.scope(),
            &namespace,
            &node_name,
            self.path.path(),
        );
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
        let write_key = ParamKey::from_scope(
            ParamIntent::Write,
            self.path.scope(),
            &namespace,
            &node_name,
            self.path.path(),
        );
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
