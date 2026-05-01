use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
struct GenericTuple<T>(T);

fn main() {}
