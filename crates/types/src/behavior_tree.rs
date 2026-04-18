use path_serde::{PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, PathSerialize, PathIntrospect)]
pub enum Status {
    Success,
    Failure,
    Idle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PathSerialize, PathIntrospect)]
pub struct NodeTrace {
    pub name: String,
    pub status: Status,
    pub children: Vec<NodeTrace>,
}
