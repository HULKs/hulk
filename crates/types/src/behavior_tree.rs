use path_serde::{PathIntrospect, PathSerialize};
use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(
    PartialEq, Debug, Clone, Serialize, Deserialize, PathSerialize, PathIntrospect, Message,
)]
pub enum Status {
    Success,
    Failure,
    Idle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PathSerialize, PathIntrospect, Message)]
pub struct NodeTrace {
    pub name: String,
    pub status: Status,
    pub children: Vec<NodeTrace>,
}
