use std::{
    mem::take,
    sync::{Arc, Mutex},
};

use color_eyre::Result;
use hardware::NetworkInterface;
use types::messages::{IncomingMessage, OutgoingMessage};

#[derive(Default)]
pub struct Interfake {
    messages: Arc<Mutex<Vec<OutgoingMessage>>>,
}

impl NetworkInterface for Interfake {
    fn read_from_network(&self) -> Result<IncomingMessage> {
        unimplemented!()
    }

    fn write_to_network(&self, message: OutgoingMessage) -> Result<()> {
        self.messages.lock().unwrap().push(message);
        Ok(())
    }
}

impl Interfake {
    pub fn take_outgoing_messages(&self) -> Vec<OutgoingMessage> {
        take(&mut self.messages.lock().unwrap())
    }
}
