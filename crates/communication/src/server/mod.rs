mod acceptor;
mod connection;
mod outputs;
mod parameters;
mod receiver;
mod runtime;
mod sender;

use std::hash::{Hash, Hasher};

pub use runtime::Runtime;
use tokio::sync::mpsc::Sender;

use crate::messages::Response;

#[derive(Clone, Debug)]
pub(crate) struct Client {
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
