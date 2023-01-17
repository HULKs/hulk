use serde_json::Value;

use crate::messages::{ParametersRequest, Path};

use super::client::Client;

pub mod directory;
pub mod storage;
pub mod subscriptions;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRequest {
    pub request: ParametersRequest,
    pub client: Client,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageRequest {
    UpdateParameter {
        client: Client,
        id: usize,
        path: Path,
        data: Value,
    },
    LoadFromDisk {
        client: Client,
        id: usize,
    },
    StoreToDisk {
        client: Client,
        id: usize,
    },
}
