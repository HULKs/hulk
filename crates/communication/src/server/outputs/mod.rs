use std::collections::BTreeSet;

use tokio::sync::mpsc::Sender;

use crate::messages::{Format, OutputsRequest, Path};

use super::client::Client;

pub mod provider;
pub mod router;

#[derive(Debug)]
pub enum Request {
    ClientRequest(ClientRequest),
    RegisterCycler {
        cycler_instance: String,
        fields: BTreeSet<Path>,
        request_sender: Sender<ClientRequest>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRequest {
    pub request: OutputsRequest,
    pub client: Client,
}

#[derive(Debug)]
struct Subscription {
    pub path: Path,
    pub format: Format,
    pub once: bool,
}
