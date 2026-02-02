//! Node - the primary unit of computation in hulkz.
//!
//! A [`Node`] represents a logical component within a session. Nodes register
//! themselves in the graph plane for discovery and provide factories for
//! publishers, subscribers, parameters, and buffers.
//!
//! # Example
//!
//! ```rust,no_run
//! # use hulkz::{Session, Result};
//! # use serde::{Serialize, Deserialize};
//! # #[derive(Serialize, Deserialize)] struct Image { data: Vec<u8> }
//! # #[derive(Clone, Serialize, Deserialize)] struct Detections { count: u32 }
//! # #[derive(Clone, Serialize, Deserialize)] struct Imu { accel: f64 }
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let session = Session::create("robot").await?;
//! let node = session.create_node("perception").build().await?;
//!
//! // Create pub/sub
//! let publisher = node.advertise::<Image>("camera/image").build().await?;
//! let subscriber = node.subscribe::<Detections>("detections").build().await?;
//!
//! // Create buffer for temporal lookups
//! let (imu, driver) = node.buffer::<Imu>("imu", 200).await?;
//! tokio::spawn(driver);
//! # Ok(())
//! # }
//! ```

use std::{future::Future, marker::PhantomData, sync::Arc};

use serde::{Deserialize, Serialize};
use zenoh::liveliness::LivelinessToken;

use crate::{
    buffer::{Buffer, BufferBuilder},
    error::{Result, ScopedPathError},
    key::graph_node_key,
    parameter::ParameterBuilder,
    publisher::PublisherBuilder,
    scoped_path::ScopedPath,
    subscriber::SubscriberBuilder,
    Session,
};


/// Builder for creating a [`Node`].
pub struct NodeBuilder {
    pub(crate) session: Session,
    pub(crate) name: String,
}

impl NodeBuilder {
    pub async fn build(self) -> Result<Node> {
        // Register node in the graph plane for discovery
        let liveliness_key = graph_node_key(self.session.namespace(), &self.name);
        let liveliness_token = self
            .session
            .zenoh()
            .liveliness()
            .declare_token(&liveliness_key)
            .await?;

        let inner = NodeInner {
            session: self.session,
            name: self.name,
            _liveliness_token: liveliness_token,
        };
        Ok(Node {
            inner: Arc::new(inner),
        })
    }
}

/// A node within a session, registered in the graph plane.
///
/// Nodes are the primary unit of computation. They register themselves
/// in the graph plane via liveliness tokens for discovery.
#[derive(Clone, Debug)]
pub struct Node {
    inner: Arc<NodeInner>,
}

#[derive(Debug)]
struct NodeInner {
    session: Session,
    name: String,
    _liveliness_token: LivelinessToken,
}

impl Node {
    const DEFAULT_CAPACITY: usize = 3;

    pub(crate) fn session(&self) -> &Session {
        &self.inner.session
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Subscribe to a topic.
    pub fn subscribe<T>(
        &self,
        topic: impl TryInto<ScopedPath, Error = impl Into<ScopedPathError>>,
    ) -> SubscriberBuilder<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        SubscriberBuilder {
            node: self.clone(),
            topic: topic.try_into().map_err(Into::into),
            capacity: Self::DEFAULT_CAPACITY,
            view_only: false,
            _phantom: PhantomData,
        }
    }

    /// Advertise a topic for publishing.
    pub fn advertise<T>(
        &self,
        topic: impl TryInto<ScopedPath, Error = impl Into<ScopedPathError>>,
    ) -> PublisherBuilder<T>
    where
        T: Serialize,
    {
        PublisherBuilder {
            node: self.clone(),
            topic: topic.try_into().map_err(Into::into),
            enable_view: true,
            _phantom: PhantomData,
        }
    }

    /// Declare a parameter with topic-like scope syntax.
    ///
    /// - `~/param` - Private (node-scoped)
    /// - `param` - Local (robot-scoped)
    /// - `/param` - Global (fleet-wide)
    pub fn declare_parameter<T>(&self, path: impl Into<String>) -> ParameterBuilder<T>
    where
        for<'de> T: Serialize + Deserialize<'de> + Clone + Send + Sync + 'static,
    {
        ParameterBuilder {
            node: self.clone(),
            path: ScopedPath::parse(&path.into()),
            default: None,
            read_only: false,
            validator: None,
            _phantom: PhantomData,
        }
    }

    /// Create a buffered subscription for temporal lookups.
    ///
    /// This is a convenience method that combines [`subscribe()`](Self::subscribe)
    /// with [`BufferBuilder`]. Returns `(Buffer<T>, driver)`. The driver future
    /// must be spawned to populate the buffer with incoming messages.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to subscribe to (supports scoped path syntax)
    /// * `capacity` - Maximum number of messages to retain in the buffer
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use hulkz::{Session, Result, Timestamp};
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Clone, Serialize, Deserialize)] struct Imu { accel: f64 }
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let session = Session::create("robot").await?;
    /// # let node = session.create_node("n").build().await?;
    /// # let camera_timestamp: Timestamp = session.now();
    /// // Create a buffered subscription
    /// let (imu, driver) = node.buffer::<Imu>("imu/data", 200).await?;
    /// tokio::spawn(driver);
    ///
    /// // Later: lookup data at a specific timestamp
    /// let msg = imu.lookup_nearest(&camera_timestamp).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn buffer<T>(
        &self,
        topic: impl TryInto<ScopedPath, Error = impl Into<ScopedPathError>>,
        capacity: usize,
    ) -> Result<(Buffer<T>, impl Future<Output = Result<()>> + Send)>
    where
        for<'de> T: Deserialize<'de> + Clone + Send + Sync + 'static,
    {
        let subscriber = self.subscribe::<T>(topic).build().await?;
        Ok(BufferBuilder::new(subscriber).capacity(capacity).build())
    }
}
