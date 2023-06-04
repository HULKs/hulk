use parameters::directory::Scope;
use serde_json::Value;

use crate::messages::Path;

use super::client::Client;

pub mod storage;
pub mod subscriptions;

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
        scope: Scope,
        path: Path,
    },
}
