use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
enum GenericEnum<T> {
    Value(T),
}

fn main() {}
