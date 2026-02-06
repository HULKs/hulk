//! Runtime-configurable parameters with read/write semantics.
//!
//! A [`Parameter`] exposes a value that can be queried and updated remotely via the param plane.
//! Parameters support validation and are loaded from configuration files on startup.
//!
//! # Scope Syntax
//!
//! - `~/param` - Private (node-scoped)
//! - `param` - Local (robot-scoped)
//! - `/param` - Global (fleet-wide)
//!
//! # Example
//!
//! ```rust,no_run
//! # use hulkz::{Session, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let session = Session::create("robot").await?;
//! # let node = session.create_node("n").build().await?;
//! let (param, driver) = node.declare_parameter::<f64>("~/max_speed")
//!     .default(1.5)
//!     .validate(|v| *v > 0.0 && *v <= 10.0)
//!     .build()
//!     .await?;
//! tokio::spawn(driver);
//!
//! let value = param.get().await;
//! # Ok(())
//! # }
//! ```

use std::{future::Future, marker::PhantomData, sync::Arc};

use cdr::{CdrLe, Infinite};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::error;
use zenoh::{
    bytes::Encoding,
    handlers::FifoChannelHandler,
    liveliness::LivelinessToken,
    pubsub::Publisher,
    query::{Query, Queryable, Reply},
};

use crate::{
    error::{Error, Result},
    key::{GraphKey, ParamIntent, ParamKey},
    raw_subscriber::RawSubscriber,
    sample::Sample,
    scoped_path::ScopedPath,
    Node, Scope, Session,
};

type ValidatorFn<T> = dyn Fn(&T) -> bool + Send + Sync;

/// Builder for creating a [`Parameter`].
pub struct ParameterBuilder<T> {
    pub(crate) node: Node,
    pub(crate) path: ScopedPath,
    pub(crate) default: Option<T>,
    pub(crate) validator: Option<Box<ValidatorFn<T>>>,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> ParameterBuilder<T>
where
    for<'de> T: Serialize + Deserialize<'de> + Clone + Send + Sync + 'static,
{
    /// Sets a default value if the parameter is not found in configuration.
    pub fn default(mut self, value: T) -> Self {
        self.default = Some(value);
        self
    }

    /// Sets a validation function for the parameter value.
    ///
    /// The function should return `true` if the value is valid.
    pub fn validate<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.validator = Some(Box::new(f));
        self
    }

    pub async fn build(self) -> Result<(Parameter<T>, impl Future<Output = Result<()>>)> {
        let ParameterBuilder {
            node,
            path,
            default,
            validator,
            _phantom,
        } = self;

        // Look up initial value from config
        let config = node.session().config();
        let config_value: Option<&Value> = match path.scope() {
            Scope::Global => config.get_global(path.path()),
            Scope::Local => config.get_local(path.path()),
            Scope::Private => config.get_private(node.name(), path.path()),
        };

        // Resolve initial value: config > default > error
        let initial: T = if let Some(value) = config_value {
            serde_json::from_value(value.clone()).map_err(Error::JsonDeserialize)?
        } else if let Some(default) = default {
            default
        } else {
            return Err(Error::ParameterNoDefault(path.path().to_string()));
        };

        // Validate initial value
        if let Some(ref validator) = validator {
            if !validator(&initial) {
                return Err(Error::ParameterValidation(format!(
                    "initial value for '{}' failed validation",
                    path.path()
                )));
            }
        }

        let namespace = node.session().namespace().to_string();
        let node_name = node.name().to_string();

        // Build key expressions using key builders
        let read_key_expr = ParamKey::from_scope(
            ParamIntent::Read,
            path.scope(),
            &namespace,
            &node_name,
            path.path(),
        );

        let z_session = node.session().zenoh();
        let value = Arc::new(Mutex::new(Arc::new(initial)));

        let reader = z_session.declare_queryable(&read_key_expr).await?;
        let broadcaster = z_session.declare_publisher(read_key_expr.clone()).await?;

        // Only create writer if not read-only
        let write_key_expr = ParamKey::from_scope(
            ParamIntent::Write,
            path.scope(),
            &namespace,
            &node_name,
            path.path(),
        );

        let writer = z_session.declare_queryable(&write_key_expr).await?;

        // Declare liveliness token for parameter discovery
        let liveliness_key = GraphKey::parameter(&namespace, &node_name, path.scope(), path.path());
        let liveliness_token = z_session
            .liveliness()
            .declare_token(&liveliness_key)
            .await?;

        let driver = {
            let value = value.clone();
            let read_key = read_key_expr.clone();
            let write_key = write_key_expr.clone();
            let validator = validator;

            async move {
                tokio::try_join!(
                    drive_reader(read_key, reader, value.clone()),
                    drive_writer(write_key, writer, value.clone(), broadcaster, validator),
                )?;
                Ok(())
            }
        };

        Ok((
            Parameter {
                value,
                _liveliness_token: liveliness_token,
            },
            driver,
        ))
    }
}

async fn drive_reader<T>(
    key_expr: String,
    reader: Queryable<FifoChannelHandler<Query>>,
    value: Arc<Mutex<Arc<T>>>,
) -> Result<()>
where
    for<'de> T: Serialize + Deserialize<'de>,
{
    loop {
        let query = reader.recv_async().await?;
        match handle_read(&query, &value).await {
            Ok((payload, encoding)) => {
                query.reply(&key_expr, payload).encoding(encoding).await?;
            }
            Err(e) => {
                error!("Failed to read parameter value: {e}");
                query
                    .reply_err(e.to_string().into_bytes())
                    .encoding(Encoding::TEXT_PLAIN)
                    .await?;
            }
        }
    }
}

async fn handle_read<T>(query: &Query, value: &Arc<Mutex<Arc<T>>>) -> Result<(Vec<u8>, Encoding)>
where
    T: Serialize,
{
    let value = {
        let guard = value.lock().await;
        guard.clone()
    };
    let encoding = query
        .encoding()
        .cloned()
        .unwrap_or(Encoding::APPLICATION_JSON);
    let payload = match encoding {
        Encoding::APPLICATION_JSON => serde_json::to_vec(&*value).map_err(Error::JsonSerialize)?,
        Encoding::APPLICATION_CDR => {
            cdr::serialize::<_, _, CdrLe>(&*value, Infinite).map_err(Error::CdrSerialize)?
        }
        encoding => {
            return Err(Error::UnsupportedEncoding(encoding));
        }
    };
    Ok((payload, encoding))
}

async fn drive_writer<T>(
    key_expr: String,
    writer: Queryable<FifoChannelHandler<Query>>,
    value: Arc<Mutex<Arc<T>>>,
    broadcaster: Publisher<'static>,
    validator: Option<Box<ValidatorFn<T>>>,
) -> Result<()>
where
    for<'de> T: Serialize + Deserialize<'de>,
{
    loop {
        let query = writer.recv_async().await?;
        match handle_write(&query, &value, validator.as_deref()).await {
            Ok(new_value) => {
                // Broadcast the new value to subscribers
                if let Err(e) = broadcast_value(&broadcaster, &new_value).await {
                    error!("Failed to broadcast parameter update: {e}");
                }

                query
                    .reply(&key_expr, b"OK".to_vec())
                    .encoding(Encoding::TEXT_PLAIN)
                    .await?;
            }
            Err(e) => {
                error!("Failed to write parameter value: {e}");
                query
                    .reply_err(e.to_string().into_bytes())
                    .encoding(Encoding::TEXT_PLAIN)
                    .await?;
            }
        }
    }
}

async fn handle_write<T>(
    query: &Query,
    value: &Arc<Mutex<Arc<T>>>,
    validator: Option<&ValidatorFn<T>>,
) -> Result<Arc<T>>
where
    for<'de> T: Deserialize<'de>,
{
    let encoding = query
        .encoding()
        .cloned()
        .unwrap_or(Encoding::APPLICATION_JSON);
    let payload = query.payload().ok_or(Error::EmptyPayload)?;
    let new_value: T = match encoding {
        Encoding::APPLICATION_JSON => {
            serde_json::from_slice(&payload.to_bytes()).map_err(Error::JsonDeserialize)?
        }
        Encoding::APPLICATION_CDR => {
            cdr::deserialize(&payload.to_bytes()).map_err(Error::CdrDeserialize)?
        }
        encoding => {
            return Err(Error::UnsupportedEncoding(encoding));
        }
    };

    // Validate if validator is present
    if let Some(validator) = validator {
        if !validator(&new_value) {
            return Err(Error::ParameterValidation(
                "new value failed validation".into(),
            ));
        }
    }

    let new_value = Arc::new(new_value);
    {
        let mut guard = value.lock().await;
        *guard = new_value.clone();
    }
    Ok(new_value)
}

async fn broadcast_value<T>(broadcaster: &Publisher<'static>, value: &T) -> Result<()>
where
    T: Serialize,
{
    let payload = serde_json::to_vec(value).map_err(Error::JsonSerialize)?;
    broadcaster
        .put(payload)
        .encoding(Encoding::APPLICATION_JSON)
        .await?;
    Ok(())
}

/// A runtime parameter that can be queried and updated remotely.
pub struct Parameter<T> {
    value: Arc<Mutex<Arc<T>>>,
    _liveliness_token: LivelinessToken,
}

impl<T> Parameter<T>
where
    for<'de> T: Deserialize<'de>,
{
    pub async fn get(&self) -> Arc<T> {
        let guard = self.value.lock().await;
        guard.clone()
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
/// let value = session.parameter("max_speed").get::<f32>().await?;
///
/// // Global parameter
/// session.parameter("/fleet_id").set(&serde_json::json!("fleet-01")).await?;
///
/// // Private parameter on specific node
/// let debug = session.parameter("~/debug").on_node("motor").get::<bool>().await?;
/// # Ok(())
/// # }
/// ```
pub struct ParamAccessBuilder<'a> {
    pub(crate) session: &'a Session,
    pub(crate) path: ScopedPath,
    pub(crate) node: Option<String>,
    pub(crate) namespace_override: Option<String>,
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
    ///     .get::<f32>()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn in_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace_override = Some(namespace.into());
        self
    }

    /// Get the parameter value.
    pub async fn get<T>(self) -> Result<ParamGetReplies<T>> {
        let ParamAccessBuilder {
            session,
            path,
            node,
            namespace_override,
        } = self;
        let node_name = Self::resolve_node(path.scope(), node)?;
        let namespace = namespace_override.unwrap_or_else(|| session.namespace().to_string());
        let key_expr = ParamKey::from_scope(
            ParamIntent::Read,
            path.scope(),
            &namespace,
            &node_name,
            path.path(),
        );

        let replies = session.zenoh().get(&key_expr).await?;
        Ok(ParamGetReplies {
            replies,
            _phantom: PhantomData,
        })
    }

    /// Set the parameter value.
    pub async fn set(self, value: &serde_json::Value) -> Result<ParamSetReplies> {
        let ParamAccessBuilder {
            session,
            path,
            node,
            namespace_override,
        } = self;
        let node_name = Self::resolve_node(path.scope(), node)?;
        let namespace = namespace_override.unwrap_or_else(|| session.namespace().to_string());
        let key_expr = ParamKey::from_scope(
            ParamIntent::Write,
            path.scope(),
            &namespace,
            &node_name,
            path.path(),
        );
        let payload = serde_json::to_vec(value).map_err(Error::JsonSerialize)?;
        let replies = session
            .zenoh()
            .get(&key_expr)
            .payload(payload)
            .encoding(Encoding::APPLICATION_JSON)
            .await?;
        Ok(ParamSetReplies { replies })
    }

    /// Subscribe to parameter updates on the param/read key.
    ///
    /// Parameters publish updates on their read key whenever the value changes (e.g. due to
    /// external writes). This method subscribes to that update stream as raw samples.
    pub async fn watch_updates_raw(self, capacity: usize) -> Result<ParamUpdateRawSubscriber> {
        let ParamAccessBuilder {
            session,
            path,
            node,
            namespace_override,
        } = self;

        let node_name = Self::resolve_node(path.scope(), node)?;
        let namespace = namespace_override.unwrap_or_else(|| session.namespace().to_string());
        let key_expr = ParamKey::from_scope(
            ParamIntent::Read,
            path.scope(),
            &namespace,
            &node_name,
            path.path(),
        );
        let inner = RawSubscriber::from_key_expr(session.clone(), key_expr, capacity).await?;
        Ok(ParamUpdateRawSubscriber { inner })
    }

    /// Resolves the node name for the key expression.
    ///
    /// - For private scope: requires explicit node, returns error if not set
    /// - For global/local scope: uses explicit node if set, otherwise wildcard
    fn resolve_node(scope: Scope, node: Option<String>) -> Result<String> {
        match (scope, node) {
            (Scope::Private, None) => Err(Error::NodeRequiredForPrivate),
            (_, Some(node)) => Ok(node),
            (_, None) => Ok("*".to_string()),
        }
    }
}

/// Receives parameter updates as raw samples.
pub struct ParamUpdateRawSubscriber {
    inner: RawSubscriber,
}

impl ParamUpdateRawSubscriber {
    pub async fn recv_async(&mut self) -> Result<Sample> {
        self.inner.recv_async().await
    }
}

/// Receives replies from a parameter get operation.
pub struct ParamGetReplies<T> {
    replies: FifoChannelHandler<Reply>,
    _phantom: PhantomData<T>,
}

impl<T> ParamGetReplies<T>
where
    for<'de> T: Deserialize<'de>,
{
    /// Receives the next parameter value reply.
    pub async fn recv_async(&mut self) -> Option<Result<T>> {
        let reply = self.replies.recv_async().await.ok()?;
        match reply.result() {
            Ok(sample) => {
                let value = serde_json::from_slice(&sample.payload().to_bytes())
                    .map_err(Error::JsonDeserialize);
                Some(value)
            }
            Err(err_reply) => {
                let reason = String::from_utf8_lossy(&err_reply.payload().to_bytes()).to_string();
                Some(Err(Error::ParameterQueryFailed(reason)))
            }
        }
    }
}

/// Receives replies from a parameter set operation.
pub struct ParamSetReplies {
    replies: FifoChannelHandler<Reply>,
}

impl ParamSetReplies {
    /// Receives the next parameter set reply.
    pub async fn recv_async(&mut self) -> Option<Result<()>> {
        let reply = self.replies.recv_async().await.ok()?;
        match reply.result() {
            Ok(_) => Some(Ok(())),
            Err(err_reply) => {
                let reason = String::from_utf8_lossy(&err_reply.payload().to_bytes()).to_string();
                Some(Err(Error::ParameterRejected(reason)))
            }
        }
    }
}
