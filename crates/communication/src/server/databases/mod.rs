use std::hash::{Hash, Hasher};

use tokio::sync::mpsc::Sender;

use super::messages::{DatabaseRequest, Format, Path, Response};

pub mod provider;
pub mod router;

#[derive(Debug)]
pub enum Request {
    ClientRequest(ClientRequest),
    RegisterCycler {
        cycler_instance: String,
        request_sender: Sender<ClientRequest>,
    },
}

#[derive(Debug)]
pub struct ClientRequest {
    pub request: DatabaseRequest,
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
    pub format: Format,
    pub once: bool,
}
