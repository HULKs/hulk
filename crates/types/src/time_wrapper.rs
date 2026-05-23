use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
pub struct TimeWrapper<T> {
    pub time: Time,
    pub inner: T,
}
