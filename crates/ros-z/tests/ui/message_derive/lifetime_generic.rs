use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
struct LifetimeGeneric<'a> {
    value: &'a str,
}

fn main() {}
