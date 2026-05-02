use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct EchoHeader {
    pub topic: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub schema_hash: String,
}

impl EchoHeader {
    pub fn new(topic: String, type_name: String, schema_hash: String) -> Self {
        Self {
            topic,
            type_name,
            schema_hash,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EchoMessageView {
    pub topic: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub schema_hash: String,
    pub data: Value,
}

impl EchoMessageView {
    pub fn new(topic: String, type_name: String, schema_hash: String, data: Value) -> Self {
        Self {
            topic,
            type_name,
            schema_hash,
            data,
        }
    }
}
