use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
struct TupleFieldMessage {
    value: (u32, u32),
}

fn main() {}
