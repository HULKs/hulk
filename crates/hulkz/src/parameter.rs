use std::{future::Future, marker::PhantomData, sync::Arc};

use cdr::{CdrLe, Infinite};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{fs::read_to_string, sync::Mutex};
use tracing::error;
use zenoh::{
    bytes::Encoding,
    handlers::FifoChannelHandler,
    query::{Query, Queryable},
};

use crate::Node;

#[derive(Error, Debug)]
pub enum ParameterError {
    #[error("failed to open file: {0}")]
    Io(#[from] std::io::Error),
    #[error("parameter not found: {0}")]
    NotFound(String),
    #[error("failed to deserialize JSON: {0}")]
    Json5(#[from] json5::Error),
    #[error("failed to serialize parameter to JSON: {0}")]
    JsonSerialize(serde_json::Error),
    #[error("failed to deserialize parameter: {0}")]
    JsonDeserialize(serde_json::Error),
    #[error("failed to serialize parameter to CDR: {0}")]
    CdrSerialize(cdr::Error),
    #[error("failed to deserialize parameter from CDR: {0}")]
    CdrDeserialize(cdr::Error),
    #[error("empty payload in query")]
    EmptyPayload,
    #[error("unsupported encoding format: {0}")]
    UnsupportedEncoding(Encoding),
    #[error("Zenoh transport error: {0}")]
    Zenoh(#[from] zenoh::Error),
}

pub type Result<T, E = ParameterError> = std::result::Result<T, E>;

pub struct ParameterBuilder<T> {
    pub(crate) node: Node,
    pub name: String,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> ParameterBuilder<T>
where
    for<'de> T: Serialize + Deserialize<'de>,
{
    pub async fn build(self) -> Result<(Parameter<T>, impl Future<Output = Result<()>>)> {
        let content = read_to_string("parameters.json5").await?;
        let parameters: serde_json::Value = json5::from_str(&content)?;

        let name = &self.name;
        let value = parameters
            .get(name)
            .ok_or_else(|| ParameterError::NotFound(self.name.clone()))?;

        let data: T =
            serde_json::from_value(value.clone()).map_err(ParameterError::JsonDeserialize)?;

        let namespace = &self.node.session().namespace();
        let key_expr = format!("{namespace}/parameters/{name}");
        let z_session = self.node.session().zenoh();
        let publisher = z_session.declare_publisher(&key_expr).await?;

        // Initial publish of the parameter value
        publisher
            .put(serde_json::to_vec(&data).map_err(ParameterError::JsonSerialize)?)
            .await?;

        let value = Arc::new(Mutex::new(Arc::new(data)));

        let getter_key_expr = format!("{key_expr}/get");
        let getter = z_session.declare_queryable(&getter_key_expr).await?;

        let setter_key_expr = format!("{key_expr}/set");
        let setter = z_session.declare_queryable(&setter_key_expr).await?;

        let driver = {
            let value = value.clone();
            async move {
                tokio::try_join!(
                    drive_getter(getter_key_expr, getter, value.clone()),
                    drive_setter(setter_key_expr, setter, value.clone()),
                )?;
                Ok(())
            }
        };

        Ok((Parameter { value }, driver))
    }
}

async fn drive_getter<T>(
    key_expr: String,
    getter: Queryable<FifoChannelHandler<Query>>,
    value: Arc<Mutex<Arc<T>>>,
) -> Result<(), ParameterError>
where
    for<'de> T: Serialize + Deserialize<'de>,
{
    loop {
        let query = getter.recv_async().await?;
        match handle_get(&query, &value).await {
            Ok((payload, encoding)) => {
                query.reply(&key_expr, payload).encoding(encoding).await?;
            }
            Err(e) => {
                error!("Failed to get parameter value: {e}");
                query
                    .reply_err(format!("Error: {e}").into_bytes())
                    .encoding(Encoding::APPLICATION_JSON)
                    .await?;
            }
        }
    }
}

async fn handle_get<T>(query: &Query, value: &Arc<Mutex<Arc<T>>>) -> Result<(Vec<u8>, Encoding)>
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
        Encoding::APPLICATION_JSON => {
            serde_json::to_vec(&*value).map_err(ParameterError::JsonSerialize)?
        }
        Encoding::APPLICATION_CDR => cdr::serialize::<_, _, CdrLe>(&*value, Infinite)
            .map_err(ParameterError::CdrSerialize)?,
        encoding => {
            return Err(ParameterError::UnsupportedEncoding(encoding));
        }
    };
    Ok((payload, encoding))
}

async fn drive_setter<T>(
    key_expr: String,
    setter: Queryable<FifoChannelHandler<Query>>,
    value: Arc<Mutex<Arc<T>>>,
) -> Result<(), ParameterError>
where
    for<'de> T: Serialize + Deserialize<'de>,
{
    loop {
        let query = setter.recv_async().await?;
        match handle_set(&query, &value).await {
            Ok(_) => {
                query
                    .reply(&key_expr, b"OK".to_vec())
                    .encoding(Encoding::APPLICATION_JSON)
                    .await?;
            }
            Err(e) => {
                error!("Failed to set parameter value: {e}");
                query
                    .reply_err(format!("Error: {e}").into_bytes())
                    .encoding(Encoding::APPLICATION_JSON)
                    .await?;
            }
        }
    }
}

async fn handle_set<T>(query: &Query, value: &Arc<Mutex<Arc<T>>>) -> Result<()>
where
    for<'de> T: Deserialize<'de>,
{
    let encoding = query
        .encoding()
        .cloned()
        .unwrap_or(Encoding::APPLICATION_JSON);
    let payload = query.payload().ok_or(ParameterError::EmptyPayload)?;
    let new_value: T = match encoding {
        Encoding::APPLICATION_JSON => {
            serde_json::from_slice(&payload.to_bytes()).map_err(ParameterError::JsonDeserialize)?
        }
        Encoding::APPLICATION_CDR => {
            cdr::deserialize(&payload.to_bytes()).map_err(ParameterError::CdrDeserialize)?
        }
        encoding => {
            return Err(ParameterError::UnsupportedEncoding(encoding));
        }
    };
    {
        let mut guard = value.lock().await;
        *guard = Arc::new(new_value);
    }
    Ok(())
}

pub struct Parameter<T> {
    value: Arc<Mutex<Arc<T>>>,
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
