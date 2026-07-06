use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, Message)]
pub enum Status {
    Success,
    Failure,
    Idle,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct NodeTrace {
    pub name: String,
    pub status: Status,
    pub children: Vec<NodeTrace>,
}
