//! Backend-independent queries over timestamped records.
mod projection;

pub use projection::{ProjectionError, TimeEncoding, TimeProjection};
