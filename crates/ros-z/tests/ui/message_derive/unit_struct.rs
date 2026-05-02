use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
struct Marker;

fn main() {
    let _ = Marker::type_name();
    let _ = Marker::schema();
    let _ = Marker::schema_hash();
}
