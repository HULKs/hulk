use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

use tokio::sync::mpsc::Sender;

use crate::messages::{Format, OutputRequest, Path, Response, Type};

pub mod provider;
pub mod router;

#[derive(Debug)]
pub enum Request {
    ClientRequest(ClientRequest),
    RegisterCycler {
        cycler_instance: String,
        fields: BTreeMap<Path, Type>,
        request_sender: Sender<ClientRequest>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRequest {
    pub request: OutputRequest,
    pub client: Client,
}

#[derive(Clone, Debug)]
pub struct Client {
    pub id: usize,
    pub response_sender: Sender<Response>,
}

impl Hash for Client {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.id.hash(state);
    }
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.response_sender.same_channel(&other.response_sender)
    }
}

impl Eq for Client {}

#[derive(Debug)]
struct Subscription {
    pub path: Path,
    #[allow(dead_code)] // TODO
    pub format: Format,
    pub once: bool,
}
