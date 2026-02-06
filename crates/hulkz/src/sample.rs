//! Raw sample wrapper.
//!
//! A [`Sample`] is the raw, timestamped payload received from Zenoh together with its encoding.
//! It abstracts over Zenoh's internal `Sample` type while still allowing efficient access to the
//! payload bytes.

use std::borrow::Cow;

use serde::Deserialize;
use tracing::warn;
use zenoh::bytes::{Encoding, ZBytes};

use crate::{
    error::{Error, Result},
    Session, Timestamp,
};

/// A received raw sample with its Zenoh timestamp and encoding.
#[derive(Clone, Debug)]
pub struct Sample {
    pub timestamp: Timestamp,
    pub encoding: Encoding,
    payload: ZBytes,
}

impl Sample {
    pub(crate) fn from_zenoh(session: &Session, sample: zenoh::sample::Sample) -> Self {
        let timestamp = sample.timestamp().copied().unwrap_or_else(|| {
            warn!("Sample has no timestamp, using current time instead");
            session.now()
        });

        Self {
            timestamp,
            encoding: sample.encoding().clone(),
            payload: sample.payload().clone(),
        }
    }

    /// Returns the raw payload bytes.
    ///
    /// If the underlying payload is not stored contiguously, Zenoh will allocate and copy.
    pub fn payload_bytes(&self) -> Cow<'_, [u8]> {
        self.payload.to_bytes()
    }

    /// Returns an iterator over the raw payload slices without forcing contiguity.
    pub fn payload_slices(&self) -> impl Iterator<Item = &[u8]> {
        self.payload.slices()
    }

    /// Returns the payload length in bytes.
    pub fn payload_len(&self) -> usize {
        self.payload.len()
    }

    /// Returns whether the payload is empty.
    pub fn payload_is_empty(&self) -> bool {
        self.payload.is_empty()
    }

    /// Decodes the payload into `T` based on the sample encoding.
    ///
    /// - `APPLICATION_CDR` -> CDR
    /// - `APPLICATION_JSON` -> JSON
    pub fn decode<T>(&self) -> Result<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        match &self.encoding {
            &Encoding::APPLICATION_CDR => {
                let bytes = self.payload.to_bytes();
                cdr::deserialize(&bytes).map_err(Error::CdrDeserialize)
            }
            &Encoding::APPLICATION_JSON => {
                let bytes = self.payload.to_bytes();
                serde_json::from_slice(&bytes).map_err(Error::JsonDeserialize)
            }
            encoding => Err(Error::UnsupportedEncoding(encoding.clone())),
        }
    }
}
