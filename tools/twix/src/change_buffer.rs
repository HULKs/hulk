use std::fmt::{Debug, Display};

use color_eyre::Result;
use color_eyre::eyre::{self, eyre};
use tokio::sync::watch;

use crate::backend::TwixTime;

#[derive(Clone, Debug)]
pub struct Change<T> {
    pub timestamp: TwixTime,
    pub source_timestamp: Option<TwixTime>,
    pub value: T,
}

#[derive(Clone)]
pub struct ChangeSeries<T> {
    changes: Vec<Change<T>>,
    first_update: Option<TwixTime>,
    last_update: Option<TwixTime>,
}

impl<T> ChangeSeries<T> {
    fn new() -> Self {
        Self {
            changes: Vec::new(),
            first_update: None,
            last_update: None,
        }
    }

    pub fn changes(&self) -> impl Iterator<Item = &Change<T>> {
        self.changes.iter()
    }

    pub fn first_update(&self) -> Option<TwixTime> {
        self.first_update
    }

    pub fn last_update(&self) -> Option<TwixTime> {
        self.last_update
    }
}

pub struct ChangeBufferHandle<T, E = eyre::Report> {
    receiver: watch::Receiver<Result<ChangeSeries<T>, E>>,
}

impl<T, E> ChangeBufferHandle<T, E>
where
    T: Clone + PartialEq,
    E: Display,
{
    pub fn get(&self) -> Result<ChangeSeries<T>> {
        let guard = self.receiver.borrow();
        guard.as_ref().map_err(|error| eyre!("{error:#}")).cloned()
    }
}

pub struct ChangeBuffer<T, E> {
    sender: watch::Sender<Result<ChangeSeries<T>, E>>,
}

impl<T: PartialEq, E> ChangeBuffer<T, E> {
    pub fn new() -> (ChangeBuffer<T, E>, ChangeBufferHandle<T, E>) {
        let (sender, receiver) = watch::channel(Ok(ChangeSeries::new()));
        let buffer = ChangeBuffer { sender };
        let handle = ChangeBufferHandle { receiver };
        (buffer, handle)
    }

    pub fn push(&self, datum: Change<T>) {
        self.sender.send_modify(|value| handle_update(value, datum));
    }

    pub fn push_error(&self, error: E) {
        let _ = self.sender.send(Err(error));
    }

    pub fn is_closed(&self) -> bool {
        self.sender.receiver_count() == 0
    }

    pub fn closed(&self) -> impl std::future::Future<Output = ()> + '_ {
        self.sender.closed()
    }
}

fn handle_update<T: PartialEq, E>(value: &mut Result<ChangeSeries<T>, E>, datum: Change<T>) {
    match value.as_mut() {
        Ok(buffer) => {
            let right = buffer
                .changes
                .partition_point(|sample| sample.timestamp < datum.timestamp);
            buffer.changes.truncate(right);

            buffer.last_update = Some(datum.timestamp);
            buffer.first_update = match buffer.first_update {
                Some(first_update) => Some(first_update.min(datum.timestamp)),
                None => Some(datum.timestamp),
            };

            if !buffer
                .changes
                .last()
                .is_some_and(|last_change| last_change.value == datum.value)
            {
                buffer.changes.push(datum);
            }
        }
        Err(_) => {
            *value = Ok(ChangeSeries {
                first_update: Some(datum.timestamp),
                last_update: Some(datum.timestamp),
                changes: vec![datum],
            });
        }
    }
}
