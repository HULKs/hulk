use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[message(name = "custom_msgs/Foo")]
struct Foo {
    value: u32,
}

fn main() {}
