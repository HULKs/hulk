use std::{any::type_name, error::Error, fmt};

use projection::camera_matrix::CameraMatrix;
use ros_z::{SerdeCdrCodec, message::WireDecoder};
use serde::de::DeserializeOwned;
use types::time_wrapper::TimeWrapper;

pub fn decode_recorded_camera_matrix(
    message: &mcap::Message<'_>,
) -> Result<TimeWrapper<CameraMatrix>, Box<dyn Error>> {
    decode_recorded_message(message)
}

pub fn decode_recorded_message<T>(message: &mcap::Message<'_>) -> Result<T, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    let decoded = SerdeCdrCodec::<T>::deserialize(message.data.as_ref()).map_err(|source| {
        RecordedMessageDecodeError {
            type_name: type_name::<T>(),
            topic: message.channel.topic.clone(),
            log_time: message.log_time,
            publish_time: message.publish_time,
            source,
        }
    })?;

    Ok(decoded)
}

#[derive(Debug)]
struct RecordedMessageDecodeError {
    type_name: &'static str,
    topic: String,
    log_time: u64,
    publish_time: u64,
    source: ros_z::message::CdrError,
}

impl fmt::Display for RecordedMessageDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to decode {} from topic '{}' at log_time={} publish_time={}",
            self.type_name, self.topic, self.log_time, self.publish_time
        )
    }
}

impl Error for RecordedMessageDecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
