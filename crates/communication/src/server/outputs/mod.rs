use std::collections::BTreeSet;

use tokio::sync::mpsc::Sender;

use crate::messages::{Format, OutputsRequest, Path};

use super::client_request::ClientRequest;

pub mod provider;
pub mod router;

#[derive(Debug)]
pub enum Request {
    ClientRequest(ClientRequest<OutputsRequest>),
    RegisterCycler {
        cycler_instance: String,
        fields: BTreeSet<Path>,
        request_sender: Sender<ClientRequest<OutputsRequest>>,
    },
}

#[derive(Debug)]
struct Subscription {
    pub path: Path,
    pub format: Format,
    pub once: bool,
}
