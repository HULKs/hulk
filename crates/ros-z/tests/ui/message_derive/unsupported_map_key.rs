use std::collections::HashMap;

use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Message,
)]
struct CustomMessageKey {
    value: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Message)]
struct InvalidMapKeyMessage {
    values: HashMap<CustomMessageKey, u32>,
}

fn main() {}
