use std::{
    env::{self, VarError},
    path::PathBuf,
    sync::{Arc, atomic::AtomicUsize},
};

use serde_json::json;
use tracing::debug;
use zenoh::{Session, Wait};

use crate::{
    Result,
    entity::normalize_node_namespace,
    error::ConfigError,
    graph::Graph,
    node::NodeBuilder,
    session_config,
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

#[derive(Default)]
pub struct ContextBuilder {
    namespace: Option<String>,
    zenoh_config: Option<zenoh::Config>,
    config_file: Option<PathBuf>,
    config_overrides: Vec<(String, serde_json::Value)>,
    enable_logging: bool,
    shm_config: Option<Arc<ShmConfig>>,
    clock: Option<Clock>,
    runtime_parameter_inputs: RuntimeParameterInputs,
}

impl ContextBuilder {
    pub fn isolated_for_test() -> Self {
        Self::default()
            .with_zenoh_config(session_config::isolated_config())
            .with_namespace("/")
    }

    /// Set the default namespace inherited by nodes created from this context.
    pub fn with_namespace(mut self, namespace: impl AsRef<str>) -> Self {
        self.namespace = Some(normalize_node_namespace(namespace.as_ref()));
        self
    }

    /// Set the explicit session configuration file.
    pub fn with_config_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config_file = Some(path.into());
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
        let value_json = session_config::serialize_override(value).map_err(|reason| {
            ConfigError::InvalidOverride {
                key: key.clone(),
                reason,
            }
        })?;
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

    /// Convenience method: connect to localhost zenohd
    pub fn connect_to_local_zenohd(self) -> Self {
        self.with_connect_endpoints(["tcp/127.0.0.1:7447"])
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

    /// Customize the default Zenoh session config to connect to a specific router endpoint.
    ///
    /// # Example
    /// ```
    /// use ros_z::context::ContextBuilder;
    ///
    /// # async fn example() -> ros_z::Result<()> {
    /// let context = ContextBuilder::default()
    ///     .with_router_endpoint("tcp/192.168.1.1:7447")?
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_router_endpoint<S: Into<String>>(self, endpoint: S) -> Result<Self> {
        let endpoint = endpoint.into();
        endpoint
            .parse::<zenoh::config::EndPoint>()
            .map_err(|_| ConfigError::InvalidOverride {
                key: "connect/endpoints".into(),
                reason: "expected a Zenoh endpoint",
            })?;
        Ok(self.with_connect_endpoints([endpoint]))
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

    #[tracing::instrument(name = "ctx_build", skip(self))]
    pub async fn build(self) -> Result<Context> {
        let namespace = resolve_namespace(self.namespace, env::var("ROS_Z_NAMESPACE"))?;
        let config = session_config::resolve(
            self.zenoh_config,
            self.config_file.as_deref(),
            self.config_overrides,
        )?;
        if self.enable_logging {
            zenoh::init_log_from_env_or("error");
        }

        let session = zenoh::open(config).await.map_err(|_| {
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

fn resolve_namespace(
    explicit: Option<String>,
    environment: std::result::Result<String, VarError>,
) -> Result<String> {
    if let Some(namespace) = explicit {
        return Ok(namespace);
    }
    match environment {
        Ok(value) => {
            let namespace = normalize_node_namespace(&value);
            validate_namespace(&namespace).map_err(|_| ConfigError::InvalidNamespaceEnv)?;
            Ok(namespace)
        }
        Err(VarError::NotPresent) => Ok(String::new()),
        Err(VarError::NotUnicode(_)) => Err(ConfigError::InvalidNamespaceEnv.into()),
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
    fn namespace_env_only_applies_without_an_explicit_context_namespace() {
        let inherited = resolve_namespace(None, Ok("/robot/42".to_string())).unwrap();
        assert_eq!(inherited, "/robot/42");
        assert!(resolve_namespace(None, Ok("/invalid/*".into())).is_err());
        assert_eq!(
            resolve_namespace(None, Err(VarError::NotPresent)).unwrap(),
            ""
        );
        for root in ["", "/"] {
            let explicit = ContextBuilder::default().with_namespace(root).namespace;
            assert_eq!(
                resolve_namespace(explicit.clone(), Ok("/from-env".into())).unwrap(),
                ""
            );
            assert_eq!(
                resolve_namespace(explicit, Ok("/invalid/*".into())).unwrap(),
                ""
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn node_namespace_overrides_the_context_namespace_even_at_root() {
        let context = ContextBuilder::isolated_for_test()
            .with_namespace("/context")
            .build()
            .await
            .unwrap();
        for (explicit, expected) in [
            (None, "/context"),
            (Some("/node"), "/node"),
            (Some("/"), ""),
        ] {
            let mut builder = context.create_node("namespaced");
            if let Some(namespace) = explicit {
                builder = builder.with_namespace(namespace);
            }
            assert_eq!(builder.build().await.unwrap().namespace(), expected);
        }
        context.shutdown().unwrap();
    }

    #[test]
    fn router_endpoint_override_preserves_the_native_base() {
        let builder = ContextBuilder::isolated_for_test()
            .with_router_endpoint("tcp/127.0.0.1:7447")
            .unwrap();
        let config =
            session_config::resolve(builder.zenoh_config, None, builder.config_overrides).unwrap();
        assert_eq!(
            config.get_json("connect/endpoints").unwrap(),
            r#"["tcp/127.0.0.1:7447"]"#
        );
        assert_eq!(
            config.get_json("listen/endpoints").unwrap(),
            r#"["tcp/127.0.0.1:0"]"#
        );
        assert_eq!(config.get_json("scouting/gossip/enabled").unwrap(), "false");
    }

    #[test]
    fn try_with_json_rejects_nested_non_finite_values_but_accepts_explicit_null() {
        #[derive(serde::Serialize)]
        enum Value {
            Nested(Vec<Option<f64>>),
        }
        for non_finite in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let error = ContextBuilder::default()
                .try_with_json("plugins/demo/value", Value::Nested(vec![Some(non_finite)]))
                .err()
                .unwrap();
            assert!(error.to_string().contains("non-finite"));
        }
        let builder = ContextBuilder::default()
            .try_with_json("plugins/demo/value", Value::Nested(vec![None, Some(1.5)]))
            .unwrap()
            .try_with_json("plugins/demo/delete", None::<f64>)
            .unwrap();
        assert_eq!(
            builder.config_overrides[0].1,
            json!({"Nested": [null, 1.5]})
        );
        assert!(builder.config_overrides[1].1.is_null());
    }

    #[test]
    fn try_with_json_preserves_json_enum_names_and_number_range() {
        #[derive(serde::Serialize)]
        enum Value {
            #[serde(rename = "!arm")]
            Arm(u8),
        }

        let builder = ContextBuilder::default()
            .try_with_json("plugins/demo/value", Value::Arm(7))
            .unwrap();
        assert_eq!(builder.config_overrides[0].1, json!({"!arm": 7}));

        let error = ContextBuilder::default()
            .try_with_json("plugins/demo/value", u128::MAX)
            .err()
            .unwrap();
        assert!(error.to_string().contains("cannot be represented as JSON"));
        assert!(!format!("{error:?}").contains(&u128::MAX.to_string()));
    }

    #[test]
    fn try_with_json_reports_serialization_error() {
        use serde::ser::{Serialize, Serializer};

        struct Fails;

        impl Serialize for Fails {
            fn serialize<S>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                Err(serde::ser::Error::custom("boom"))
            }
        }

        let error = match ContextBuilder::default().try_with_json("bad/key", Fails) {
            Ok(_) => panic!("serialization failure should be returned"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("bad/key"));
        assert!(!format!("{error:?}").contains("boom"));
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
