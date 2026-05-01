use std::{sync::Arc, time::Duration};

use crate::{
    ServiceTypeInfo,
    action::{client::ActionClientBuilder, server::ActionServerBuilder},
    cache::CacheBuilder,
    context::{GlobalCounter, RemapRules, RuntimeParameterInputs},
    dynamic::{
        DiscoveredTopicSchema, DynamicCdrCodec, DynamicError, DynamicMessage,
        DynamicPublisherBuilder, DynamicSubscriberBuilder, MessageSchema, SchemaDiscovery,
        SchemaService, discovered_schema_type_info, schema_service::SchemaServiceNodeIdentity,
        schema_type_info,
    },
    entity::*,
    graph::Graph,
    msg::{Message, Service, WireMessage},
    pubsub::{DEFAULT_TRANSIENT_LOCAL_REPLAY_TIMEOUT, PublisherBuilder, SubscriberBuilder},
    service::{ServiceClientBuilder, ServiceServerBuilder},
};
use tracing::{debug, info, warn};
use zenoh::{Result, Session, liveliness::LivelinessToken};

/// A native ros-z node: a named participant that owns publishers, subscribers,
/// service clients, service servers, and action clients/servers.
///
/// Create a node via [`Context::create_node`](crate::context::Context::create_node):
///
/// ```rust,ignore
/// use ros_z::prelude::*;
///
/// let context = ContextBuilder::default().build().await?;
/// let node = context.create_node("my_node").build().await?;
/// ```
pub struct Node {
    pub(crate) entity: NodeEntity,
    pub(crate) session: Arc<Session>,
    counter: Arc<GlobalCounter>,
    pub(crate) graph: Arc<Graph>,
    pub(crate) remap_rules: RemapRules,
    _lv_token: LivelinessToken,
    pub(crate) clock: crate::time::Clock,
    pub(crate) shm_config: Option<Arc<crate::shm::ShmConfig>>,
    runtime_parameter_inputs: RuntimeParameterInputs,
    parameter_binding_state: Arc<parking_lot::Mutex<bool>>,
    /// Optional schema service for this node.
    /// Enabled by default and disabled via `NodeBuilder::without_schema_service()`.
    /// The service uses callback mode and requires no background task.
    schema_service: Option<SchemaService>,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("entity", &self.entity)
            .finish_non_exhaustive()
    }
}

pub struct NodeBuilder {
    pub(crate) domain_id: usize,
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) enclave: String,
    pub(crate) session: Arc<Session>,
    pub(crate) counter: Arc<GlobalCounter>,
    pub(crate) graph: Arc<Graph>,
    pub(crate) remap_rules: RemapRules,
    pub(crate) clock: crate::time::Clock,
    pub(crate) shm_config: Option<Arc<crate::shm::ShmConfig>>,
    pub(crate) runtime_parameter_inputs: RuntimeParameterInputs,
    /// Whether this node should expose its default schema service.
    pub(crate) enable_schema_service: bool,
}

impl NodeBuilder {
    pub fn with_namespace<S: AsRef<str>>(mut self, namespace: S) -> Self {
        self.namespace = normalize_node_namespace(namespace.as_ref());
        self
    }

    /// Append one parameter layer to the inherited node parameter layer list.
    pub fn with_parameter_layer<P: Into<std::path::PathBuf>>(mut self, path: P) -> Self {
        self.runtime_parameter_inputs
            .parameter_layers
            .push(path.into());
        self
    }

    /// Replace the inherited node parameter layer list entirely.
    pub fn with_parameter_layers<I, P>(mut self, layers: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<std::path::PathBuf>,
    {
        self.runtime_parameter_inputs.parameter_layers =
            layers.into_iter().map(Into::into).collect();
        self
    }

    /// Override SHM configuration for this node (and its publishers).
    ///
    /// This overrides the context-level SHM configuration for all publishers
    /// created from this node.
    ///
    /// # Example
    /// ```no_run
    /// use ros_z::shm::{ShmConfig, ShmProviderBuilder};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> zenoh::Result<()> {
    /// # let context = ros_z::context::ContextBuilder::default().build().await?;
    /// let provider = Arc::new(ShmProviderBuilder::new(20 * 1024 * 1024).build()?);
    /// let config = ShmConfig::new(provider).with_threshold(5_000);
    ///
    /// let node = context.create_node("my_node")
    ///     .with_shm_config(config)
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_shm_config(mut self, config: crate::shm::ShmConfig) -> Self {
        self.shm_config = Some(Arc::new(config));
        self
    }

    /// Disable the schema service for this node.
    ///
    /// By default, the node exposes a `~get_schema` service
    /// that allows other nodes to query schemas
    /// registered with this node's publishers.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let node = context
    ///     .create_node("my_node")
    ///     .without_schema_service()
    ///     .build()
    ///     .await?;
    /// ```
    pub fn without_schema_service(mut self) -> Self {
        self.enable_schema_service = false;
        self
    }
}

impl NodeBuilder {
    #[tracing::instrument(name = "node_build", skip(self), fields(
        name = %self.name,
        namespace = %self.namespace,
        id = tracing::field::Empty
    ))]
    pub async fn build(self) -> Result<Node> {
        let id = self.counter.increment();
        tracing::Span::current().record("id", id);

        debug!(
            "[NOD] Creating node: {}/{}, id={}",
            self.namespace, self.name, id
        );

        let node = NodeEntity::new(
            self.domain_id,
            self.session.zid(),
            id,
            self.name.clone(),
            self.namespace.clone(),
            self.enclave,
        );
        let liveliness_token_key_expr = crate::entity::node_lv_token_key_expr(&node)?;
        debug!("[NOD] Liveliness token KE: {}", liveliness_token_key_expr);

        let lv_token = self
            .session
            .liveliness()
            .declare_token(liveliness_token_key_expr)
            .await?;

        // Create schema service if enabled
        let schema_service = if self.enable_schema_service {
            debug!("[NOD] Creating schema service");
            let service = SchemaService::new(
                self.session.clone(),
                SchemaServiceNodeIdentity {
                    domain_id: self.domain_id,
                    name: &self.name,
                    namespace: &self.namespace,
                    id,
                },
                &self.counter,
                &self.clock,
            )
            .await?;

            info!("[NOD] SchemaService created (callback mode)");

            Some(service)
        } else {
            None
        };

        debug!("[NOD] Node ready: {}/{}", self.namespace, self.name);

        Ok(Node {
            entity: node,
            session: self.session,
            counter: self.counter,
            _lv_token: lv_token,
            graph: self.graph,
            remap_rules: self.remap_rules,
            clock: self.clock,
            shm_config: self.shm_config,
            runtime_parameter_inputs: self.runtime_parameter_inputs,
            parameter_binding_state: Arc::new(parking_lot::Mutex::new(false)),
            schema_service,
        })
    }
}

impl Node {
    /// Create a publisher for the given topic.
    ///
    /// If `T` implements [`Message`], type information is automatically populated.
    /// If this node has schema service enabled and `T` provides a runtime
    /// schema via [`crate::Message::schema`], that schema is
    /// automatically registered for `GetSchema` discovery.
    ///
    /// The topic name will be qualified as a ros-z graph name:
    /// - Absolute topics (starting with '/') are used as-is
    /// - Private topics (starting with '~') are expanded to /<namespace>/<node_name>/<topic>
    /// - Relative topics are expanded to /<namespace>/<topic>
    pub fn publisher<T>(&self, topic: &str) -> PublisherBuilder<T>
    where
        T: Message,
        T::Codec: crate::msg::WireEncoder,
    {
        debug!("[NOD] Creating publisher: topic={}", topic);
        let schema = T::schema();
        let mut builder = self.publisher_impl::<T, T::Codec>(
            topic,
            Some(TypeInfo::with_hash(T::type_name(), T::schema_hash())),
        );
        if let Err(error) = self.register_schema_with_service(Arc::clone(&schema)) {
            warn!(
                "[NOD] Failed to register schema {} with schema service: {}",
                schema.type_name_str(),
                error
            );
        }
        builder = builder.dyn_schema(schema);

        builder
    }

    /// Create a publisher for a message type that does not implement [`Message`].
    ///
    /// This is the explicit escape hatch for publishing a [`crate::msg::WireMessage`] with
    /// manually supplied DDS type metadata. Unlike [`Self::publisher`], this does not
    /// auto-register a runtime schema with the schema service.
    ///
    /// Use this when the caller already knows the advertised ROS type name and hash but
    /// intentionally does not have a discoverable [`crate::dynamic::MessageSchema`].
    pub fn publisher_with_type_info<T>(
        &self,
        topic: &str,
        type_info: Option<crate::entity::TypeInfo>,
    ) -> PublisherBuilder<T, T::Codec>
    where
        T: WireMessage,
    {
        self.publisher_impl::<T, T::Codec>(topic, type_info)
    }

    fn publisher_impl<T, S>(
        &self,
        topic: &str,
        type_info: Option<crate::entity::TypeInfo>,
    ) -> PublisherBuilder<T, S>
    where
        S: crate::msg::WireEncoder,
    {
        // Note: Topic qualification happens in PublisherBuilder::build()
        // to allow error handling in the Result type
        let entity = EndpointEntity {
            id: self.counter.increment(),
            node: Some(self.entity.clone()),
            kind: EndpointKind::Publisher,
            topic: topic.to_string(),
            type_info,
            qos: Default::default(),
        };
        PublisherBuilder {
            entity,
            session: self.session.clone(),
            graph: self.graph.clone(),
            clock: self.clock.clone(),
            attachment: true,
            shm_config: self.shm_config.clone(),
            dyn_schema: None,
            _phantom_data: Default::default(),
        }
    }

    /// Create a subscriber for the given topic
    /// If T implements Message, type information will be automatically populated
    ///
    /// The topic name will be qualified as a ros-z graph name:
    /// - Absolute topics (starting with '/') are used as-is
    /// - Private topics (starting with '~') are expanded to /<namespace>/<node_name>/<topic>
    /// - Relative topics are expanded to /<namespace>/<topic>
    pub fn subscriber<T>(&self, topic: &str) -> SubscriberBuilder<T>
    where
        T: Message,
        T::Codec: crate::msg::WireDecoder,
    {
        debug!("[NOD] Creating subscriber: topic={}", topic);
        self.subscriber_impl::<T, T::Codec>(
            topic,
            Some(TypeInfo::with_hash(T::type_name(), T::schema_hash())),
        )
    }

    /// Create a subscriber for a message type that does not implement [`Message`].
    ///
    /// This mirrors [`Self::publisher_with_type_info`] for callers that need to provide
    /// manual DDS type metadata while opting out of schema-based discovery.
    pub fn subscriber_with_type_info<T>(
        &self,
        topic: &str,
        type_info: Option<crate::entity::TypeInfo>,
    ) -> SubscriberBuilder<T, T::Codec>
    where
        T: WireMessage,
    {
        self.subscriber_impl::<T, T::Codec>(topic, type_info)
    }

    fn subscriber_impl<T, S>(
        &self,
        topic: &str,
        type_info: Option<crate::entity::TypeInfo>,
    ) -> SubscriberBuilder<T, S>
    where
        S: crate::msg::WireDecoder,
    {
        // Note: Topic qualification happens in SubscriberBuilder::build()
        // to allow error handling in the Result type
        let entity = EndpointEntity {
            id: self.counter.increment(),
            node: Some(self.entity.clone()),
            kind: EndpointKind::Subscription,
            topic: topic.to_string(),
            type_info,
            qos: Default::default(),
        };
        SubscriberBuilder {
            entity,
            session: self.session.clone(),
            graph: self.graph.clone(),
            dyn_schema: None,
            locality: None,
            transient_local_replay_timeout: DEFAULT_TRANSIENT_LOCAL_REPLAY_TIMEOUT,
            _phantom_data: Default::default(),
        }
    }

    /// Create a timestamp-indexed sliding-window cache subscriber for `topic`,
    /// retaining up to `capacity` messages.
    ///
    /// A capacity of `0` disables retention and stores no messages.
    ///
    /// By default, messages are indexed by the Zenoh transport timestamp
    /// (zero-config, works for any message type). Call
    /// [`.with_stamp(|message| ...)`](CacheBuilder::with_stamp) on the returned
    /// builder to switch to application-level timestamp extraction (e.g.
    /// `header.stamp`).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ros_z::prelude::*;
    /// use ros_z::time::Time;
    /// use ros_z_msgs::sensor_msgs::Imu;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> zenoh::Result<()> {
    /// let context = ContextBuilder::default().build().await?;
    /// let node = context.create_node("cache_demo").build().await?;
    ///
    /// // Zero-config (Zenoh transport timestamp)
    /// let cache = node.create_cache::<Imu>("/imu/data", 200).build().await?;
    ///
    /// // Pull messages from the last 100 ms
    /// let now = Time::from_wallclock(std::time::SystemTime::now());
    /// let msgs = cache.get_interval(now - Duration::from_millis(100), now);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_cache<T>(
        &self,
        topic: &str,
        capacity: usize,
    ) -> CacheBuilder<T, <T as Message>::Codec>
    where
        T: WireMessage + Message,
        for<'a> <T as Message>::Codec: crate::msg::WireDecoder<Input<'a> = &'a [u8], Output = T>,
    {
        debug!(
            "[NOD] Creating cache: topic={}, capacity={}",
            topic, capacity
        );
        let sub_builder =
            self.subscriber_impl::<T, <T as Message>::Codec>(topic, Some(T::type_info()));
        CacheBuilder::new(sub_builder, capacity)
    }

    /// Create a periodic timer tied to this node's clock.
    ///
    /// This is a thin convenience wrapper around [`crate::time::Clock::timer`]
    /// so node code can express periodic work directly from the node handle.
    pub fn create_timer(&self, period: impl Into<Duration>) -> crate::time::Timer {
        self.clock.timer(period)
    }

    /// Create a typed service server builder for `name`.
    ///
    /// `T` must implement [`Service`] to define the request/response pair and
    /// [`ServiceTypeInfo`] to provide the advertised ROS service type metadata.
    ///
    /// The service name will be qualified as a ros-z graph name:
    /// - Absolute service names (starting with '/') are used as-is
    /// - Private service names (starting with '~') are expanded to /<namespace>/<node_name>/<service>
    /// - Relative service names are expanded to /<namespace>/<service>
    pub fn create_service_server<T>(&self, name: &str) -> ServiceServerBuilder<T>
    where
        T: Service + ServiceTypeInfo,
    {
        debug!("[NOD] Creating service server: name={}", name);
        self.create_service_impl(name, Some(T::service_type_info()))
    }

    #[doc(hidden)]
    pub fn create_service_impl<T>(
        &self,
        name: &str,
        type_info: Option<crate::entity::TypeInfo>,
    ) -> ServiceServerBuilder<T> {
        // Note: Service name qualification happens in ServiceServerBuilder::build()
        // to allow error handling in the Result type
        let entity = EndpointEntity {
            id: self.counter.increment(),
            node: Some(self.entity.clone()),
            kind: EndpointKind::Service,
            topic: name.to_string(),
            type_info,
            qos: Default::default(),
        };
        ServiceServerBuilder {
            entity,
            session: self.session.clone(),
            clock: self.clock.clone(),
            _phantom_data: Default::default(),
        }
    }

    /// Create a typed service client builder for `name`.
    ///
    /// `T` must implement [`Service`] to define the request/response pair and
    /// [`ServiceTypeInfo`] to provide the advertised ROS service type metadata.
    ///
    /// The service name will be qualified as a ros-z graph name:
    /// - Absolute service names (starting with '/') are used as-is
    /// - Private service names (starting with '~') are expanded to /<namespace>/<node_name>/<service>
    /// - Relative service names are expanded to /<namespace>/<service>
    pub fn create_service_client<T>(&self, name: &str) -> ServiceClientBuilder<T>
    where
        T: Service + ServiceTypeInfo,
    {
        debug!("[NOD] Creating service client: name={}", name);
        self.create_client_impl(name, Some(T::service_type_info()))
    }

    #[doc(hidden)]
    pub fn create_client_impl<T>(
        &self,
        name: &str,
        type_info: Option<crate::entity::TypeInfo>,
    ) -> ServiceClientBuilder<T> {
        // Note: Service name qualification happens in ServiceClientBuilder::build()
        // to allow error handling in the Result type
        let entity = EndpointEntity {
            id: self.counter.increment(),
            node: Some(self.entity.clone()),
            kind: EndpointKind::Client,
            topic: name.to_string(),
            type_info,
            qos: Default::default(),
        };
        ServiceClientBuilder {
            entity,
            session: self.session.clone(),
            clock: self.clock.clone(),
            _phantom_data: Default::default(),
        }
    }

    /// Create an action client for the given action name
    pub fn create_action_client<A>(&self, action_name: &str) -> ActionClientBuilder<'_, A>
    where
        A: crate::action::Action,
    {
        ActionClientBuilder::new(action_name, self)
    }

    /// Create an action server for the given action name
    pub fn create_action_server<A>(&self, action_name: &str) -> ActionServerBuilder<'_, A>
    where
        A: crate::action::Action,
    {
        ActionServerBuilder::new(action_name, self)
    }

    /// Get a reference to this node's schema service, if enabled.
    ///
    /// Returns `None` if the node was created with `.without_schema_service()`.
    pub fn schema_service(&self) -> Option<&SchemaService> {
        self.schema_service.as_ref()
    }

    /// Get a mutable reference to this node's schema service, if enabled.
    ///
    /// Returns `None` if the node was created with `.without_schema_service()`.
    pub fn schema_service_mut(&mut self) -> Option<&mut SchemaService> {
        self.schema_service.as_mut()
    }

    /// Check if this node has a schema service.
    pub fn has_schema_service(&self) -> bool {
        self.schema_service.is_some()
    }

    /// Get access to the global counter for entity ID generation.
    pub fn counter(&self) -> &Arc<GlobalCounter> {
        &self.counter
    }

    /// Get the name of this node.
    pub fn name(&self) -> &str {
        &self.entity.name
    }

    /// Get the namespace of this node.
    pub fn namespace(&self) -> &str {
        &self.entity.namespace
    }

    /// Get a reference to the graph for this node.
    pub fn graph(&self) -> &Arc<Graph> {
        &self.graph
    }

    /// Get a reference to the node entity (for graph and liveliness operations).
    pub fn node_entity(&self) -> &NodeEntity {
        &self.entity
    }

    /// Apply remapping rules to a topic or action name.
    pub fn apply_remap(&self, name: &str) -> String {
        self.remap_rules.apply(name)
    }

    /// Get a reference to the underlying Zenoh session.
    pub fn session(&self) -> &Arc<Session> {
        &self.session
    }

    /// Access parameter-related startup inputs inherited from the context.
    pub fn runtime_parameter_inputs(&self) -> &RuntimeParameterInputs {
        &self.runtime_parameter_inputs
    }

    /// Internal coordination state used by external parameter subsystems.
    #[doc(hidden)]
    pub fn parameter_binding_state(&self) -> &Arc<parking_lot::Mutex<bool>> {
        &self.parameter_binding_state
    }

    /// Access this node's clock.
    pub fn clock(&self) -> &crate::time::Clock {
        &self.clock
    }

    // ========================================================================
    // Dynamic Message API
    // ========================================================================

    /// Create a dynamic publisher for the given topic.
    ///
    /// If this node has a schema service enabled, the schema will be
    /// automatically registered, allowing other nodes to discover it via the
    /// `GetSchema` service.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name to publish on
    /// * `schema` - The message schema for serialization
    ///
    /// # Example
    ///
    /// ```ignore
    /// let schema = MessageSchema::builder("std_msgs::String")
    ///     .field("data", FieldType::String)
    ///     .build()?;
    ///
    /// let publisher = node.dynamic_publisher("chatter", schema).build().await?;
    ///
    /// let mut message = DynamicMessage::new(publisher.schema());
    /// message.set("data", "Hello, world!")?;
    /// publisher.publish(&message).await?;
    /// ```
    pub fn dynamic_publisher(
        &self,
        topic: &str,
        schema: Arc<MessageSchema>,
    ) -> DynamicPublisherBuilder {
        let schema_hash = match self.advertised_schema_hash_for_schema(&schema) {
            Ok(hash) => Some(hash),
            Err(error) => {
                warn!(
                    "[NOD] Failed to compute schema hash for {}: {}",
                    schema.type_name_str(),
                    error
                );
                None
            }
        };
        if let Err(error) = self.register_schema_with_service(Arc::clone(&schema)) {
            warn!(
                "[NOD] Failed to register schema {} with schema service: {}",
                schema.type_name_str(),
                error
            );
        }
        self.dynamic_publisher_impl(
            topic,
            Some(TypeInfo {
                name: schema.type_name_str().to_string(),
                hash: schema_hash,
            }),
            schema,
        )
    }

    /// Discover the schema that publishers currently expose on a topic.
    ///
    /// The topic name is qualified according to the same ros-z graph-name rules as the
    /// regular publisher and subscriber builder APIs.
    pub async fn discover_topic_schema(
        &self,
        topic: &str,
        discovery_timeout: Duration,
    ) -> std::result::Result<DiscoveredTopicSchema, crate::dynamic::DynamicError> {
        SchemaDiscovery::new(self, discovery_timeout)
            .discover(topic)
            .await
    }

    /// Create a dynamic subscriber with automatic schema discovery.
    ///
    /// This method queries publishers on the topic for their schema service
    /// and returns a preconfigured subscriber builder using the discovered
    /// schema. This is useful when you don't know the message type at compile
    /// time.
    ///
    /// The topic name will be qualified as a ros-z graph name:
    /// - Absolute topics (starting with '/') are used as-is
    /// - Private topics (starting with '~') are expanded to /<namespace>/<node_name>/<topic>
    /// - Relative topics are expanded to /<namespace>/<topic>
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name to subscribe to
    /// * `discovery_timeout` - How long to wait for schema discovery
    ///
    /// # Returns
    ///
    /// A preconfigured dynamic subscriber builder on success.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Discover schema from publishers and create subscriber
    /// let subscriber = node.dynamic_subscriber_auto(
    ///     "chatter",
    ///     Duration::from_secs(5),
    /// ).await?
    /// .build()
    /// .await?;
    ///
    /// println!("Discovered type: {}", subscriber.schema().unwrap().type_name);
    ///
    /// // Receive messages
    /// let message = subscriber.recv().await?;
    /// let data: String = message.get("data")?;
    /// ```
    pub async fn dynamic_subscriber_auto(
        &self,
        topic: &str,
        discovery_timeout: Duration,
    ) -> Result<DynamicSubscriberBuilder> {
        debug!(
            "[NOD] Creating dynamic subscriber with auto-discovery for topic: {}",
            topic
        );

        let discovered = self
            .discover_topic_schema(topic, discovery_timeout)
            .await
            .map_err(|error| zenoh::Error::from(error.to_string()))?;

        info!(
            "[NOD] Discovered schema for topic {}: {} (hash: {})",
            discovered.qualified_topic,
            discovered.schema.type_name_str(),
            discovered.schema_hash.to_hash_string()
        );

        Ok(self.dynamic_subscriber_impl(
            topic,
            Some(discovered_schema_type_info(&discovered)),
            discovered.schema,
        ))
    }

    /// Create a dynamic subscriber with a known schema.
    ///
    /// Use this when you already have the schema (e.g., loaded from a file
    /// or built programmatically).
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name to subscribe to
    /// * `schema` - The message schema for deserialization
    ///
    /// The topic name will be qualified as a ros-z graph name:
    /// - Absolute topics (starting with '/') are used as-is
    /// - Private topics (starting with '~') are expanded to /<namespace>/<node_name>/<topic>
    /// - Relative topics are expanded to /<namespace>/<topic>
    ///
    /// # Example
    ///
    /// ```ignore
    /// let schema = MessageSchema::builder("std_msgs::String")
    ///     .field("data", FieldType::String)
    ///     .build()?;
    ///
    /// let subscriber = node.dynamic_subscriber("chatter", schema).build().await?;
    /// let message = subscriber.recv().await?;
    /// ```
    pub fn dynamic_subscriber(
        &self,
        topic: &str,
        schema: Arc<MessageSchema>,
    ) -> DynamicSubscriberBuilder {
        self.dynamic_subscriber_impl(topic, Some(schema_type_info(&schema)), schema)
    }

    fn dynamic_publisher_impl(
        &self,
        topic: &str,
        type_info: Option<crate::entity::TypeInfo>,
        schema: Arc<MessageSchema>,
    ) -> DynamicPublisherBuilder {
        self.publisher_impl::<DynamicMessage, DynamicCdrCodec>(topic, type_info)
            .dyn_schema(schema)
    }

    fn dynamic_subscriber_impl(
        &self,
        topic: &str,
        type_info: Option<crate::entity::TypeInfo>,
        schema: Arc<MessageSchema>,
    ) -> DynamicSubscriberBuilder {
        self.subscriber_impl::<DynamicMessage, DynamicCdrCodec>(topic, type_info)
            .dyn_schema(schema)
    }

    pub fn register_schema_with_service(
        &self,
        schema: Arc<MessageSchema>,
    ) -> std::result::Result<(), DynamicError> {
        if let Some(service) = &self.schema_service {
            service.register_schema(Arc::clone(&schema))?;
            debug!(
                "[NOD] Registered schema {} with schema service",
                schema.type_name_str()
            );
        }

        Ok(())
    }

    pub(crate) fn advertised_schema_hash_for_schema(
        &self,
        schema: &MessageSchema,
    ) -> std::result::Result<SchemaHash, DynamicError> {
        crate::dynamic::type_info::schema_hash(schema).ok_or_else(|| {
            DynamicError::SerializationError(
                "runtime schema conversion must produce a schema hash".to_string(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_entity_name_namespace() {
        let entity = NodeEntity::new(
            0,
            "1234567890abcdef1234567890abcdef".parse().unwrap(),
            0,
            "my_node".to_string(),
            "/my_ns".to_string(),
            String::new(),
        );
        assert_eq!(entity.name, "my_node");
        assert_eq!(entity.namespace, "/my_ns");
    }

    #[test]
    fn test_remap_rules_identity_when_empty() {
        let rules = RemapRules::default();
        assert_eq!(rules.apply("/foo"), "/foo");
    }
}
