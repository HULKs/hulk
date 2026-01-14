use std::time::Duration;

pub use crate::buffer::TopicBuffer;
pub use crate::cache::Cache;
pub use crate::publisher::Publisher;
pub use crate::session::Session;
pub use crate::stream::TopicStream;

pub mod buffer;
pub mod cache;
pub mod publisher;
pub mod session;
pub mod stream;

pub trait Timestamped {
    fn timestamp(&self) -> Duration;
}
