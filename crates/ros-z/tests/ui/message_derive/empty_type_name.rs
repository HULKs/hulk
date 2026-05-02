use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[message(name = "")]
struct EmptyTypeName {
    value: u32,
}

fn main() {}
