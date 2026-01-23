use serde::{Deserialize, Serialize};

use crate::Timestamp;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message<T> {
    pub timestamp: Timestamp,
    pub payload: T,
}
