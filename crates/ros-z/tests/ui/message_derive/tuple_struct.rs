use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
struct TupleStatus(f32, f32);

fn main() {}
