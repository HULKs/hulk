use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
struct ConstGeneric<const N: usize> {
    value: u32,
}

fn main() {}
