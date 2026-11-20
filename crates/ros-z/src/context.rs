use std::{
    env::VarError,
    path::PathBuf,
    sync::{Arc, atomic::AtomicUsize},
};

use serde_json::json;
use tracing::debug;
use zenoh::{Session, Wait};

use crate::{
    Result, config,
    entity::normalize_node_namespace,
    error::ConfigError,
    graph::Graph,
    node::NodeBuilder,
    shm::{DEFAULT_SHM_POOL_SIZE, ShmConfig, ShmProviderBuilder},
    time::Clock,
    topic_name::validate_namespace,
};

#[derive(Debug, Default)]
pub struct GlobalCounter(AtomicUsize);

impl GlobalCounter {
    pub fn increment(&self) -> usize {
        self.0.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }
}

/// Parameter-related startup inputs carried from the context builder into nodes.
///
/// These are intentionally plain values so external crates can build richer
/// subsystems on top of `ros-z` without creating dependency cycles back into the
/// core crate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeParameterInputs {
    pub parameter_layers: Vec<PathBuf>,
}

/// Builds a ROS-Z context. Session settings are layered in this order: SDK defaults,
/// the file named by `ZENOH_SESSION_CONFIG_URI`, `ROS_Z_GATEWAY`, then the file
/// passed to `with_config_file`.
///
/// `with_zenoh_config` supplies a complete configuration instead. In either case,
/// builder field overrides apply last.
#[derive(Default)]
pub struct ContextBuilder {
    namespace: Option<String>,
    zenoh_config: Option<zenoh::Config>,
    config_overlay: Option<PathBuf>,
    config_overrides: Vec<(String, serde_json::Value)>,
    enable_logging: bool,
    shm_config: Option<Arc<ShmConfig>>,
    clock: Option<Clock>,
    runtime_parameter_inputs: RuntimeParameterInputs,
}

impl ContextBuilder {
    pub fn isolated_for_test() -> Self {
        Self::default()
            .with_zenoh_config(config::isolated_config())
            .with_namespace("/")
    }

    /// Set the namespace inherited by nodes, overriding `ROS_Z_NAMESPACE`.
    /// An empty string or `/` explicitly selects the root namespace.
    pub fn with_namespace(mut self, namespace: impl AsRef<str>) -> Self {
        self.namespace = Some(normalize_node_namespace(namespace.as_ref()));
        self
    }

    pub fn with_config_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config_overlay = Some(path.into());
        self
    }

    /// Add a JSON configuration override
    ///
    /// # Example
    /// ```
    /// use serde_json::json;
    /// use ros_z::context::ContextBuilder;
    ///
    /// # async fn example() -> ros_z::Result<()> {
    /// let context = ContextBuilder::default()
    ///     .with_json("scouting/multicast/enabled", json!(false))
    ///     .with_json("connect/endpoints", json!(["tcp/127.0.0.1:7447"]))
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_json<K: Into<String>, V: serde::Serialize>(self, key: K, value: V) -> Self {
        self.try_with_json(key, value)
            .expect("Failed to serialize value for config override")
    }

    pub fn try_with_json<K, V>(mut self, key: K, value: V) -> Result<Self>
    where
        K: Into<String>,
        V: serde::Serialize,
    {
        let key = key.into();
        let value_json = serde_json::to_value(value)
            .map_err(|_| ConfigError::SerializeOverride { key: key.clone() })?;
        self.config_overrides.push((key, value_json));
        Ok(self)
    }

    /// Convenience method: disable multicast scouting
    pub fn disable_multicast_scouting(self) -> Self {
        self.with_json("scouting/multicast/enabled", json!(false))
    }

    /// Convenience method: connect to specific endpoints
    ///
    /// # Example
    /// ```
    /// use ros_z::context::ContextBuilder;
    ///
    /// # async fn example() -> ros_z::Result<()> {
    /// let context = ContextBuilder::default()
    ///     .with_connect_endpoints(["tcp/127.0.0.1:7447"])
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_connect_endpoints<I, S>(self, endpoints: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let endpoints: Vec<String> = endpoints.into_iter().map(|s| s.into()).collect();
        self.with_json("connect/endpoints", json!(endpoints))
    }

    /// Convenience method: listen on specific endpoints
    ///
    /// Overrides the listeners of whichever native configuration is selected.
    ///
    /// # Example
    /// ```
    /// use ros_z::context::ContextBuilder;
    ///
    /// # async fn example() -> ros_z::Result<()> {
    /// let context = ContextBuilder::default()
    ///     .with_listen_endpoints(["tcp/[::]:0"])
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_listen_endpoints<I, S>(self, endpoints: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let endpoints: Vec<String> = endpoints.into_iter().map(|s| s.into()).collect();
        self.with_json("listen/endpoints", json!(endpoints))
    }

    /// Convenience method: set mode (peer, client, router)
    pub fn with_mode<S: Into<String>>(self, mode: S) -> Self {
        self.with_json("mode", json!(mode.into()))
    }

    /// Override the default Zenoh session config with a custom Zenoh configuration.
    ///
    /// # Example
    /// ```
    /// use ros_z::context::ContextBuilder;
    ///
    /// # async fn example() -> ros_z::Result<()> {
    /// let custom_config = zenoh::Config::default();
    /// let context = ContextBuilder::default()
    ///     .with_zenoh_config(custom_config)
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_zenoh_config(mut self, config: zenoh::Config) -> Self {
        self.zenoh_config = Some(config);
        self
    }

    /// Enable Zenoh logging initialization with default level "error"
    pub fn with_logging_enabled(mut self) -> Self {
        self.enable_logging = true;
        self
    }

    /// Inject a pre-configured clock.
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = Some(clock);
        self
    }

    /// Append one parameter layer used by external parameter subsystems.
    pub fn with_parameter_layer<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.runtime_parameter_inputs
            .parameter_layers
            .push(path.into());
        self
    }

    /// Replace the active parameter layers used by external parameter subsystems.
    pub fn with_parameter_layers<I, P>(mut self, layers: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.runtime_parameter_inputs.parameter_layers =
            layers.into_iter().map(Into::into).collect();
        self
    }

    /// Enable SHM with default pool size (10MB) and threshold (512 bytes).
    ///
    /// # Example
    /// ```no_run
    /// use ros_z::context::ContextBuilder;
    ///
    /// # async fn example() -> ros_z::Result<()> {
    /// let context = ContextBuilder::default()
    ///     .with_shm_enabled()?
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_shm_enabled(self) -> Result<Self> {
        let provider = Arc::new(ShmProviderBuilder::new(DEFAULT_SHM_POOL_SIZE).build()?);
        Ok(ContextBuilder::with_shm_config(
            self,
            ShmConfig::new(provider),
        ))
    }

    /// Enable SHM with custom pool size.
    ///
    /// # Arguments
    /// * `size_bytes` - Pool size in bytes
    ///
    /// # Example
    /// ```no_run
    /// use ros_z::context::ContextBuilder;
    ///
    /// # async fn example() -> ros_z::Result<()> {
    /// let context = ContextBuilder::default()
    ///     .with_shm_pool_size(100 * 1024 * 1024)?  // 100MB
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_shm_pool_size(self, size_bytes: usize) -> Result<Self> {
        let provider = Arc::new(ShmProviderBuilder::new(size_bytes).build()?);
        Ok(ContextBuilder::with_shm_config(
            self,
            ShmConfig::new(provider),
        ))
    }

    /// Set custom SHM configuration.
    ///
    /// # Example
    /// ```no_run
    /// use ros_z::context::ContextBuilder;
    /// use ros_z::shm::{ShmConfig, ShmProviderBuilder};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> ros_z::Result<()> {
    /// let provider = Arc::new(ShmProviderBuilder::new(50 * 1024 * 1024).build()?);
    /// let config = ShmConfig::new(provider).with_threshold(10_000);
    ///
    /// let context = ContextBuilder::default()
    ///     .with_shm_config(config)
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_shm_config(mut self, config: ShmConfig) -> Self {
        self.shm_config = Some(Arc::new(config));
        self
    }

    /// Set SHM threshold (minimum message size for SHM).
    ///
    /// Only effective if SHM has been enabled via `with_shm_enabled()` or similar.
    ///
    /// # Example
    /// ```no_run
    /// use ros_z::context::ContextBuilder;
    ///
    /// # async fn example() -> ros_z::Result<()> {
    /// let context = ContextBuilder::default()
    ///     .with_shm_enabled()?
    ///     .with_shm_threshold(50_000)  // 50KB threshold
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_shm_threshold(mut self, threshold: usize) -> Self {
        if let Some(ref mut config) = self.shm_config {
            // Need to modify Arc content - make it unique or clone
            let mut new_config = (**config).clone();
            new_config = new_config.with_threshold(threshold);
            self.shm_config = Some(Arc::new(new_config));
        }
        self
    }

    fn resolve_config(&mut self) -> Result<zenoh::Config> {
        let mut config = match self.zenoh_config.take() {
            Some(config) => config,
            None => config::session_config(self.config_overlay.as_deref())?,
        };
        for (key, value) in &self.config_overrides {
            config
                .insert_json5(key, &value.to_string())
                .map_err(|_| ConfigError::InvalidOverride { key: key.clone() })?;
        }
        Ok(config)
    }

    #[tracing::instrument(name = "ctx_build", skip(self))]
    pub async fn build(mut self) -> Result<Context> {
        let namespace = match self.namespace.take() {
            Some(namespace) => {
                validate_namespace(&namespace)
                    .map_err(|source| crate::Error::namespace(&namespace, source))?;
                namespace
            }
            None => namespace_from_env(std::env::var("ROS_Z_NAMESPACE"))?,
        };

        if self.enable_logging {
            zenoh::init_log_from_env_or("error");
        }
        let config = self.resolve_config()?;

        // Open Zenoh session
        let session = zenoh::open(config).await.map_err(|_| {
            // Native open errors can include credential-bearing endpoints.
            crate::Error::zenoh(
                "open Zenoh session",
                "check the configured endpoints, credentials and transport settings".into(),
            )
        })?;
        debug!("[CTX] Zenoh session opened: zid={}", session.zid());

        let graph = Arc::new(Graph::new(&session).await?);

        Ok(Context {
            session,
            counter: Arc::new(GlobalCounter::default()),
            namespace,
            graph,
            shm_config: self.shm_config,
            clock: self.clock.unwrap_or_default(),
            runtime_parameter_inputs: self.runtime_parameter_inputs,
        })
    }
}

fn namespace_from_env(environment: std::result::Result<String, VarError>) -> Result<String> {
    match environment {
        Ok(value) => {
            let namespace = normalize_node_namespace(&value);
            validate_namespace(&namespace)
                .map_err(|source| ConfigError::InvalidNamespaceEnv { value, source })?;
            Ok(namespace)
        }
        Err(VarError::NotPresent) => Ok(String::new()),
        Err(source) => Err(ConfigError::NamespaceEnv { source }.into()),
    }
}

/// A live ros-z context backed by an open Zenoh session.
///
/// `Context` is the root object for all ros-z communication. Create one with
/// [`ContextBuilder`] and use it to create [`Node`](crate::node::Node)s.
///
/// # Example
///
/// ```rust,ignore
/// use ros_z::prelude::*;
///
/// let context = ContextBuilder::default().build().await?;
/// let node = context.create_node("my_node").build().await?;
/// ```
#[derive(Clone)]
pub struct Context {
    pub(crate) session: Session,
    // Global counter for the participants
    counter: Arc<GlobalCounter>,
    namespace: String,
    graph: Arc<Graph>,
    pub(crate) shm_config: Option<Arc<ShmConfig>>,
    pub(crate) clock: Clock,
    runtime_parameter_inputs: RuntimeParameterInputs,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("namespace", &self.namespace)
            .finish_non_exhaustive()
    }
}

impl Context {
    /// Call `.build().await` on the returned [`NodeBuilder`] to
    /// produce the node.
    pub fn create_node<S: AsRef<str>>(&self, name: S) -> NodeBuilder {
        NodeBuilder {
            name: name.as_ref().to_owned(),
            namespace: self.namespace.clone(),
            session: self.session.clone(),
            counter: self.counter.clone(),
            graph: self.graph.clone(),
            shm_config: self.shm_config.clone(),
            clock: self.clock.clone(),
            runtime_parameter_inputs: self.runtime_parameter_inputs.clone(),
            enable_schema_service: true,
        }
    }

    /// Close the underlying Zenoh session, releasing all network resources.
    ///
    /// After calling `shutdown`, all nodes, publishers, subscribers, and
    /// service clients/servers created from this context become invalid.
    pub fn shutdown(&self) -> Result<()> {
        self.session
            .close()
            .wait()
            .map_err(|source| crate::Error::zenoh("close Zenoh session", source))
    }

    /// Get a reference to the Zenoh session for advanced use cases. For most uses, this should not
    /// be necessary.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get a reference to the graph for setting up event callbacks
    pub fn graph(&self) -> &Arc<Graph> {
        &self.graph
    }

    /// Access the context clock used by nodes and runtime helpers.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Access parameter-related startup inputs carried by this context.
    pub fn runtime_parameter_inputs(&self) -> &RuntimeParameterInputs {
        &self.runtime_parameter_inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_from_env_accepts_valid_namespaces_and_defaults_to_root() {
        assert_eq!(
            namespace_from_env(Ok("/robot/43".into())).unwrap(),
            "/robot/43"
        );
        assert_eq!(namespace_from_env(Err(VarError::NotPresent)).unwrap(), "");
        for root in ["", "/"] {
            assert_eq!(namespace_from_env(Ok(root.into())).unwrap(), "");
        }
    }

    #[test]
    fn invalid_namespace_env_is_an_error_not_root_fallback() {
        for invalid in ["/robot//43", "/robot/", "/robot%43", "/robot/*"] {
            let error = namespace_from_env(Ok(invalid.into())).unwrap_err();
            assert!(error.to_string().contains("ROS_Z_NAMESPACE"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;

            let error = namespace_from_env(Err(VarError::NotUnicode(
                std::ffi::OsString::from_vec(vec![0xff]),
            )))
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("ROS_Z_NAMESPACE must contain valid Unicode")
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn nodes_inherit_context_namespace_and_can_override_it() {
        let context = ContextBuilder::isolated_for_test()
            .with_namespace("/robot/43")
            .build()
            .await
            .unwrap();
        let inherited = context.create_node("inherited").build().await.unwrap();
        let explicit = context
            .create_node("explicit")
            .with_namespace("/node")
            .build()
            .await
            .unwrap();
        let root = context
            .create_node("root")
            .with_namespace("/")
            .build()
            .await
            .unwrap();
        assert_eq!(inherited.namespace(), "/robot/43");
        assert_eq!(explicit.namespace(), "/node");
        assert_eq!(root.namespace(), "");
        context.shutdown().unwrap();
    }

    #[test]
    fn explicit_native_config_bypasses_missing_overlay_in_either_call_order() {
        let dir = tempfile::tempdir().unwrap();
        let overlay = dir.path().join("missing-explicit.json5");
        let native = config::isolated_config();
        for mut builder in [
            ContextBuilder::default()
                .with_zenoh_config(native.clone())
                .with_config_file(&overlay),
            ContextBuilder::default()
                .with_config_file(&overlay)
                .with_zenoh_config(native),
        ] {
            assert!(builder.resolve_config().is_ok());
        }
    }

    #[test]
    fn json_overrides_apply_after_explicit_native_config_in_call_order() {
        let mut native = config::isolated_config();
        native
            .insert_json5("queries_default_timeout", "500")
            .unwrap();
        let config = ContextBuilder::default()
            .with_json("queries_default_timeout", 1000)
            .with_zenoh_config(native)
            .with_json("queries_default_timeout", 2000)
            .resolve_config()
            .unwrap();
        assert_eq!(config.get_json("queries_default_timeout").unwrap(), "2000");
    }

    #[test]
    fn rejected_json_override_does_not_include_value_in_error() {
        let error = ContextBuilder::isolated_for_test()
            .with_mode("private-override-value")
            .resolve_config()
            .unwrap_err();
        assert!(error.to_string().contains("mode"));
        assert!(!format!("{error}: {error:?}").contains("private-override-value"));
    }

    #[test]
    fn try_with_json_reports_serialization_error_without_value() {
        use serde::ser::{Serialize, Serializer};

        const SECRET: &str = "private-serialized-value";
        struct Fails;

        impl Serialize for Fails {
            fn serialize<S>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                Err(serde::ser::Error::custom(SECRET))
            }
        }

        let error = match ContextBuilder::default().try_with_json("bad/key", Fails) {
            Ok(_) => panic!("serialization failure should be returned"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("could not serialize"));
        assert!(!format!("{error}: {error:?}").contains(SECRET));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn context_uses_replaced_parameter_layers() {
        let context = ContextBuilder::isolated_for_test()
            .with_parameter_layer("./parameters/base")
            .with_parameter_layer("./parameters/robot")
            .with_parameter_layers(["./parameters/override"])
            .build()
            .await
            .unwrap();

        assert_eq!(
            context.runtime_parameter_inputs().parameter_layers,
            vec![PathBuf::from("./parameters/override")]
        );

        context.shutdown().unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn node_uses_replaced_parameter_layers() {
        let context = ContextBuilder::isolated_for_test()
            .with_parameter_layer("./parameters/base")
            .build()
            .await
            .unwrap();

        let node = context
            .create_node("demo")
            .with_parameter_layer("./parameters/robot")
            .with_parameter_layers(["./parameters/override"])
            .build()
            .await
            .unwrap();

        assert_eq!(
            node.runtime_parameter_inputs().parameter_layers,
            vec![PathBuf::from("./parameters/override")]
        );

        context.shutdown().unwrap();
    }
}
