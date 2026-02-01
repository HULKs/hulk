//! Publisher for dual-plane (data + view) message publishing.
//!
//! A [`Publisher`] sends messages to both the data plane (CDR encoding for
//! performance) and optionally the view plane (JSON for debugging). View plane
//! serialization is lazy - it only occurs when subscribers are present.
//!
//! # Timestamps
//!
//! [`Publisher::put`] requires an explicit timestamp. This ensures correct
//! temporal semantics:
//!
//! - **Sensor data**: Use `session.now()` (capture time)
//! - **Derived data**: Use source message's timestamp (temporal coherence)
//!
//! # Example
//!
//! ```rust,no_run
//! # use hulkz::{Session, Result};
//! # use serde::{Serialize, Deserialize};
//! # #[derive(Serialize, Deserialize)] struct Odometry { x: f64 }
//! # #[derive(Clone, Serialize, Deserialize)] struct Input { v: f64 }
//! # #[derive(Serialize, Deserialize)] struct Filtered { v: f64 }
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let session = Session::create("robot").await?;
//! # let node = session.create_node("n").build().await?;
//! # let reading = Odometry { x: 1.0 };
//! # let filtered = Filtered { v: 1.0 };
//! # let mut sub = node.subscribe::<Input>("in").build().await?;
//! # let input_msg = sub.recv_async().await?;
//! let publisher = node.advertise::<Odometry>("odom").build().await?;
//!
//! // Sensor: use current time
//! publisher.put(&reading, &session.now()).await?;
//!
//! // Filter: inherit source timestamp
//! let filter_pub = node.advertise::<Filtered>("filtered").build().await?;
//! filter_pub.put(&filtered, &input_msg.timestamp).await?;
//! # Ok(())
//! # }
//! ```

use cdr::{CdrLe, Infinite};
use serde::Serialize;
use std::marker::PhantomData;
use tracing::debug;
use zenoh::{bytes::Encoding, liveliness::LivelinessToken, pubsub::Publisher as ZenohPublisher};

use crate::{
    error::{Error, Result, ScopedPathError},
    scoped_path::ScopedPath,
    Node, Timestamp,
};

/// Builder for creating a [`Publisher`].
pub struct PublisherBuilder<T>
where
    T: Serialize,
{
    pub(crate) node: Node,
    pub topic: Result<ScopedPath, ScopedPathError>,
    pub enable_view: bool,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> PublisherBuilder<T>
where
    T: Serialize,
{
    /// Enables publishing to the View plane (JSON mirror for debugging).
    ///
    /// This is enabled by default. The View plane uses lazy serialization:
    /// JSON is only serialized if there are subscribers on the view key.
    pub fn enable_view(mut self) -> Self {
        self.enable_view = true;
        self
    }

    /// Disables publishing to the View plane.
    ///
    /// Use this for high-frequency topics where the JSON overhead is undesirable
    /// even with lazy serialization.
    pub fn disable_view(mut self) -> Self {
        self.enable_view = false;
        self
    }

    pub async fn build(self) -> Result<Publisher<T>> {
        let topic = self.topic?;

        let publisher = self
            .node
            .session()
            .zenoh()
            .declare_publisher(topic.to_data_key(self.node.session().namespace(), self.node.name()))
            .await?;

        let view_publisher = if self.enable_view {
            let key_expression =
                topic.to_view_key(self.node.session().namespace(), self.node.name());
            Some(
                self.node
                    .session()
                    .zenoh()
                    .declare_publisher(key_expression)
                    .await?,
            )
        } else {
            None
        };

        // Declare liveliness token for publisher discovery
        let liveliness_key =
            topic.to_graph_publisher_key(self.node.session().namespace(), self.node.name());
        let liveliness_token = self
            .node
            .session()
            .zenoh()
            .liveliness()
            .declare_token(&liveliness_key)
            .await?;

        Ok(Publisher {
            publisher,
            view_publisher,
            _liveliness_token: liveliness_token,
            _phantom: PhantomData,
        })
    }
}

/// Publishes to the data plane (CDR) and optionally the view plane (JSON).
///
/// The view plane uses lazy serialization: JSON is only serialized when
/// subscribers are present on the view key.
pub struct Publisher<T>
where
    T: Serialize,
{
    publisher: ZenohPublisher<'static>,
    view_publisher: Option<ZenohPublisher<'static>>,
    _liveliness_token: LivelinessToken,
    _phantom: PhantomData<T>,
}

impl<T> Publisher<T>
where
    T: Serialize,
{
    pub async fn is_subscribed(&self) -> Result<bool> {
        let cdr_matching = self.is_cdr_subscribed().await?;
        let view_matching = self.is_view_subscribed().await?;
        Ok(cdr_matching || view_matching)
    }

    async fn is_cdr_subscribed(&self) -> Result<bool> {
        let cdr_matching = self.publisher.matching_status().await?.matching();
        Ok(cdr_matching)
    }

    async fn is_view_subscribed(&self) -> Result<bool> {
        let view_matching = if let Some(view_publisher) = &self.view_publisher {
            view_publisher.matching_status().await?.matching()
        } else {
            false
        };
        Ok(view_matching)
    }

    /// Publishes a value with an explicit timestamp.
    ///
    /// The timestamp should represent when the data was captured or computed.
    /// For sensor data, use the sensor's capture time. For derived data (e.g.,
    /// filtered IMU), use the source data's timestamp to maintain temporal
    /// coherence.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use hulkz::{Session, Result};
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)] struct SensorReading { v: f64 }
    /// # #[derive(Clone, Serialize, Deserialize)] struct ImuData { v: f64 }
    /// # #[derive(Serialize, Deserialize)] struct Filtered { v: f64 }
    /// # struct Filter; impl Filter { fn update(&self, _: &ImuData) -> Filtered { Filtered { v: 0.0 } } }
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let session = Session::create("robot").await?;
    /// # let node = session.create_node("n").build().await?;
    /// # let publisher = node.advertise::<SensorReading>("sensor").build().await?;
    /// # let filter_pub = node.advertise::<Filtered>("filtered").build().await?;
    /// # let sensor_reading = SensorReading { v: 1.0 };
    /// # let filter = Filter;
    /// # let mut sub = node.subscribe::<ImuData>("imu").build().await?;
    /// # let imu_msg = sub.recv_async().await?;
    /// // For sensor data: use current time
    /// let timestamp = session.now();
    /// publisher.put(&sensor_reading, &timestamp).await?;
    ///
    /// // For derived data: use source timestamp
    /// let filtered = filter.update(&imu_msg.payload);
    /// filter_pub.put(&filtered, &imu_msg.timestamp).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn put(&self, value: &T, timestamp: &Timestamp) -> Result<()> {
        debug!("Publishing value to topic");
        let payload =
            cdr::serialize::<_, _, CdrLe>(value, Infinite).map_err(Error::CdrSerialize)?;

        self.publisher
            .put(payload)
            .encoding(Encoding::APPLICATION_CDR)
            .timestamp(timestamp.clone())
            .await?;
        self.put_view(value, timestamp).await?;
        Ok(())
    }

    async fn put_view(&self, value: &T, timestamp: &Timestamp) -> Result<()> {
        let Some(view_publisher) = &self.view_publisher else {
            return Ok(());
        };

        let is_matched = view_publisher.matching_status().await?.matching();
        if !is_matched {
            return Ok(());
        }

        let json_payload = serde_json::to_vec(value).map_err(Error::JsonSerialize)?;
        view_publisher
            .put(json_payload)
            .encoding(Encoding::APPLICATION_JSON)
            .timestamp(timestamp.clone())
            .await?;
        Ok(())
    }

    /// Publishes a value only if there are subscribers, with an explicit timestamp.
    ///
    /// The value closure is only called if there are active subscribers.
    pub async fn put_if_subscribed(
        &self,
        timestamp: &Timestamp,
        mut value: impl FnMut() -> T,
    ) -> Result<()> {
        if self.is_subscribed().await? {
            let value = value();
            self.put(&value, timestamp).await?;
        }
        Ok(())
    }
}
