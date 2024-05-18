use std::{
    borrow::BorrowMut,
    mem::take,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use buffered_watch::{ReaderGuard, Receiver, Sender};
use color_eyre::Result;
use hardware::{NetworkInterface, RecordingInterface, TimeInterface};
use types::messages::{IncomingMessage, OutgoingMessage};

use crate::{cyclers::control::Database, HardwareInterface};

pub struct Interfake {
    time: SystemTime,
    messages: Arc<Mutex<Vec<OutgoingMessage>>>,
    last_database_receiver: Mutex<Receiver<Database>>,
    last_database_sender: Mutex<Sender<Database>>,
}

impl Default for Interfake {
    fn default() -> Self {
        let (last_database_sender, last_database_receiver) =
            buffered_watch::channel(Default::default());
        Self {
            time: UNIX_EPOCH,
            messages: Default::default(),
            last_database_receiver: Mutex::new(last_database_receiver),
            last_database_sender: Mutex::new(last_database_sender),
        }
    }
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

impl RecordingInterface for Interfake {
    fn should_record(&self) -> bool {
        false
    }

    fn set_whether_to_record(&self, _enable: bool) {}
}

impl TimeInterface for Interfake {
    fn get_now(&self) -> SystemTime {
        self.time
    }
}

pub trait FakeDataInterface {
    fn get_last_database_receiver(&self) -> &Mutex<Receiver<Database>>;
    fn get_last_database_sender(&self) -> &Mutex<Sender<Database>>;
}

impl FakeDataInterface for Interfake {
    fn get_last_database_receiver(&self) -> &Mutex<Receiver<Database>> {
        &self.last_database_receiver
    }

    fn get_last_database_sender(&self) -> &Mutex<Sender<Database>> {
        &self.last_database_sender
    }
}

impl Interfake {
    pub fn take_outgoing_messages(&self) -> Vec<OutgoingMessage> {
        take(&mut self.messages.lock().unwrap())
    }
}

impl HardwareInterface for Interfake {}
