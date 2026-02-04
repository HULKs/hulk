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
    query::{Query, Queryable},
};

use crate::{
    error::{Error, Result},
    key::{ParamIntent, Scope},
    scoped_path::ScopedPath,
    Node,
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
        // Look up initial value from config
        let config = self.node.session().config();
        let config_value: Option<&Value> = match self.path.scope() {
            Scope::Global => config.get_global(self.path.path()),
            Scope::Local => config.get_local(self.path.path()),
            Scope::Private => config.get_private(self.node.name(), self.path.path()),
        };

        // Resolve initial value: config > default > error
        let initial: T = if let Some(value) = config_value {
            serde_json::from_value(value.clone()).map_err(Error::JsonDeserialize)?
        } else if let Some(default) = self.default {
            default
        } else {
            return Err(Error::ParameterNoDefault(self.path.path().to_string()));
        };

        // Validate initial value
        if let Some(ref validator) = self.validator {
            if !validator(&initial) {
                return Err(Error::ParameterValidation(format!(
                    "initial value for '{}' failed validation",
                    self.path.path()
                )));
            }
        }

        let namespace = self.node.session().namespace().to_string();
        let node_name = self.node.name().to_string();

        // Build key expressions using ScopedPath
        let read_key_expr = self
            .path
            .to_param_key(ParamIntent::Read, &namespace, &node_name);

        let z_session = self.node.session().zenoh();
        let value = Arc::new(Mutex::new(Arc::new(initial)));

        let reader = z_session.declare_queryable(&read_key_expr).await?;
        let broadcaster = z_session.declare_publisher(read_key_expr.clone()).await?;

        // Only create writer if not read-only
        let write_key_expr = self
            .path
            .to_param_key(ParamIntent::Write, &namespace, &node_name);

        let writer = z_session.declare_queryable(&write_key_expr).await?;

        // Declare liveliness token for parameter discovery
        let liveliness_key = self.path.to_graph_parameter_key(&namespace, &node_name);
        let liveliness_token = z_session
            .liveliness()
            .declare_token(&liveliness_key)
            .await?;

        let driver = {
            let value = value.clone();
            let read_key = read_key_expr.clone();
            let write_key = write_key_expr.clone();
            let validator = self.validator;

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
                    .reply_err(format!("Error: {e}").into_bytes())
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
                    .reply_err(format!("Error: {e}").into_bytes())
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
