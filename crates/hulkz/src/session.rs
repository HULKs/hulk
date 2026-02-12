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
//! let nodes = session.graph().nodes().list().await?;
//! let (watcher, driver) = session.graph().nodes().watch().await?;
//! tokio::spawn(driver);
//! # Ok(())
//! # }
//! ```

use std::{path::PathBuf, sync::Arc};

use tracing::{debug, info};
use zenoh::liveliness::LivelinessToken;

use crate::{
    key::GraphKey, node::NodeBuilder, Config, GraphAccess, ParamAccessBuilder, Result, ScopedPath,
    Timestamp,
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
        info!(
            namespace = %self.namespace,
            config_overlays = self.config_files.len(),
            "opening new Zenoh session",
        );

        // Load config: environment/convention first, then explicit files
        self.config = Config::load_default().await?;
        for path in &self.config_files {
            debug!(path = %path.display(), "loading session parameter overlay");
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
        info!(
            namespace = %self.namespace,
            session_id = %session_id,
            "zenoh session opened and liveliness token declared",
        );

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

    pub fn config(&self) -> &Config {
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
    ///         GraphEvent::Joined(info) => println!("+ {}", info.name),
    ///         GraphEvent::Left(info) => println!("- {}", info.name),
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
    /// let value = session.parameter("max_speed").get::<f32>().await?;
    ///
    /// // Set global parameter
    /// session.parameter("/fleet_id").set(&serde_json::json!("fleet-01")).await?;
    ///
    /// // Get private parameter (requires node)
    /// let debug = session.parameter("~/debug").on_node("motor").get::<bool>().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn parameter(&self, path: impl Into<ScopedPath>) -> ParamAccessBuilder<'_> {
        ParamAccessBuilder {
            session: self,
            path: path.into(),
            node: None,
            namespace_override: None,
        }
    }
}
