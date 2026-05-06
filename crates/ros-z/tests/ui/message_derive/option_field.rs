use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
struct OptionField {
    value: Option<u32>,
}

fn main() {}
