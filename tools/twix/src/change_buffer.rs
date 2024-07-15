use std::{
    fmt::{Debug, Display},
    time::SystemTime,
};

use color_eyre::eyre::{self, eyre};
use color_eyre::Result;
use communication::client::{
    protocol::{self, SubscriptionEvent},
    SubscriptionHandle,
};
use tokio::{select, sync::watch};

#[derive(Clone, Debug)]
pub struct Change<T> {
    pub timestamp: SystemTime,
    pub value: T,
}

#[derive(Clone)]
pub struct ChangeSeries<T> {
    changes: Vec<Change<T>>,
    first_update: Option<SystemTime>,
    last_update: Option<SystemTime>,
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

    pub fn first_update(&self) -> Option<SystemTime> {
        self.first_update
    }

    pub fn last_update(&self) -> Option<SystemTime> {
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

    pub async fn map<U: Debug>(
        self,
        mut subscription: SubscriptionHandle<U>,
        op: impl Fn(Result<Change<&U>, &protocol::Error>) -> Result<Change<T>, E>
            + Send
            + Sync
            + 'static,
    ) {
        loop {
            select! {
                maybe_event = subscription.receiver.recv() => {
                    match maybe_event {
                        Ok(event) => {
                            let maybe_datum = match event.as_ref() {
                                SubscriptionEvent::Successful { timestamp, value } => Ok(Change {
                                    timestamp: *timestamp,
                                    value,
                                }),
                                SubscriptionEvent::Update { timestamp, value } => Ok(Change {
                                    timestamp: *timestamp,
                                    value,
                                }),
                                SubscriptionEvent::Failure { error } => Err(error),
                            };
                            let maybe_datum = op(maybe_datum);
                            match maybe_datum {
                                Ok(datum) => {
                                    self.sender.send_modify(|value| handle_update(value, datum))
                                }
                                Err(error) => {
                                    let _ = self.sender.send(Err(error));
                                }
                            }
                        },
                        Err(_) => break,
                    }
                },
                _ = self.sender.closed() => {
                    break
                }
            };
        }
    }
}

fn handle_update<T: PartialEq, E>(value: &mut Result<ChangeSeries<T>, E>, datum: Change<T>) {
    match value {
        Ok(ref mut buffer) => {
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
