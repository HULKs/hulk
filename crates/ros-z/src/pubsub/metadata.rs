use std::ops::Deref;

use tracing::warn;
use zenoh::sample::Sample;

use crate::attachment::{Attachment, EndpointGlobalId};
use crate::time::Time;

/// A deserialized message together with the transport and source timestamps seen
/// by the receiver.
#[derive(Debug, Clone)]
pub struct Received<T> {
    pub message: T,
    pub transport_time: Option<Time>,
    pub source_time: Option<Time>,
    pub sequence_number: Option<i64>,
    pub source_global_id: Option<EndpointGlobalId>,
}

/// Unique identifier for one publication emitted by a specific publisher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicationId {
    endpoint_global_id: EndpointGlobalId,
    sequence_number: i64,
}

impl PublicationId {
    pub(crate) fn new(endpoint_global_id: EndpointGlobalId, sequence_number: i64) -> Self {
        Self {
            endpoint_global_id,
            sequence_number,
        }
    }

    pub fn endpoint_global_id(&self) -> EndpointGlobalId {
        self.endpoint_global_id
    }

    pub fn sequence_number(&self) -> i64 {
        self.sequence_number
    }
}

pub(super) fn publication_id_from_sample(sample: &Sample) -> Option<PublicationId> {
    sample
        .attachment()
        .and_then(|raw| Attachment::try_from(raw).ok())
        .map(|attachment| {
            PublicationId::new(attachment.source_global_id, attachment.sequence_number)
        })
}

impl<T> Received<T> {
    pub(super) fn from_sample(sample: &Sample, message: T) -> Self {
        let transport_time = sample
            .timestamp()
            .map(|ts| Time::from_wallclock(ts.get_time().to_system_time()));

        let attachment = match sample.attachment() {
            Some(raw) => match Attachment::try_from(raw) {
                Ok(attachment) => Some(attachment),
                Err(err) => {
                    warn!("[SUB] Failed to decode attachment metadata: {}", err);
                    None
                }
            },
            None => None,
        };

        Self {
            message,
            transport_time,
            source_time: attachment.as_ref().map(Attachment::source_time),
            sequence_number: attachment.as_ref().map(|att| att.sequence_number),
            source_global_id: attachment.as_ref().map(|att| att.source_global_id),
        }
    }

    pub fn message(&self) -> &T {
        &self.message
    }

    pub fn into_message(self) -> T {
        self.message
    }

    /// Return the publication id carried in transport attachment metadata.
    pub fn publication_id(&self) -> Option<PublicationId> {
        match (self.source_global_id, self.sequence_number) {
            (Some(source_global_id), Some(sequence_number)) => {
                Some(PublicationId::new(source_global_id, sequence_number))
            }
            _ => None,
        }
    }
}

impl<T> Deref for Received<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl<T: PartialEq> PartialEq<T> for Received<T> {
    fn eq(&self, other: &T) -> bool {
        self.message == *other
    }
}

impl<T: PartialEq> PartialEq for Received<T> {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
            && self.transport_time == other.transport_time
            && self.source_time == other.source_time
            && self.sequence_number == other.sequence_number
            && self.source_global_id == other.source_global_id
    }
}
