use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[repr(C)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Request {
    uuid: String,
    header: String,
    body: String,
}

impl Request {
    pub fn new(header: impl Into<String>, body: impl Into<String>) -> Self {
        let uuid = Uuid::new_v4().to_string();
        let header = header.into();
        let body = body.into();

        Self { uuid, header, body }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Response {
    uuid: String,
    header: String,
    body: String,
}
