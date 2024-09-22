use std::{
    fmt::{Debug, Display},
    iter::once,
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::eyre::{self, eyre};
use color_eyre::Result;
use communication::client::{
    protocol::{self, SubscriptionEvent},
    SubscriptionHandle,
};
use tokio::{
    select,
    sync::{watch, Mutex},
};

#[derive(Clone, Debug)]
pub struct Datum<T> {
    pub timestamp: SystemTime,
    pub value: T,
}

type TimeSeries<T> = Vec<Datum<T>>;

#[derive(Clone)]
pub struct BufferHandle<T, E = eyre::Report> {
    receiver: watch::Receiver<Result<TimeSeries<T>, E>>,
    history: Arc<Mutex<Duration>>,
}

impl<T, E> BufferHandle<T, E>
where
    T: Clone,
    E: Display,
{
    pub fn get(&self) -> Result<TimeSeries<T>> {
        let guard = self.receiver.borrow();
        guard.as_ref().map_err(|error| eyre!("{error:#}")).cloned()
    }

    pub fn get_last(&self) -> Result<Option<Datum<T>>> {
        let guard = self.receiver.borrow();
        match guard.as_ref() {
            Ok(series) => Ok(series.last().cloned()),
            Err(error) => Err(eyre!("{error:#}")),
        }
    }

    pub fn has_changed(&self) -> bool {
        self.receiver.has_changed().unwrap()
    }

    pub fn mark_as_seen(&mut self) {
        self.receiver.mark_unchanged();
    }

    pub fn get_last_timestamp(&self) -> Result<Option<SystemTime>> {
        let guard = self.receiver.borrow();
        match guard.as_ref() {
            Ok(series) => Ok(series.last().map(|datum| datum.timestamp)),
            Err(error) => Err(eyre!("{error:#}")),
        }
    }

    pub fn get_last_value(&self) -> Result<Option<T>> {
        Ok(self.get_last()?.map(|datum| datum.value))
    }

    pub fn set_history(&self, history: Duration) {
        *self.history.blocking_lock() = history;
    }
}

pub struct Buffer<T, E> {
    sender: watch::Sender<Result<TimeSeries<T>, E>>,
    history: Arc<Mutex<Duration>>,
}

impl<T, E> Buffer<T, E> {
    pub fn new(history: Duration) -> (Buffer<T, E>, BufferHandle<T, E>) {
        let (sender, receiver) = watch::channel(Ok(TimeSeries::new()));
        let history = Arc::new(Mutex::new(history));
        let buffer = Buffer {
            sender,
            history: history.clone(),
        };
        let handle = BufferHandle { receiver, history };
        (buffer, handle)
    }

    pub async fn map<U: Debug>(
        self,
        mut subscription: SubscriptionHandle<U>,
        op: impl Fn(Result<Datum<&U>, &protocol::Error>) -> Result<Datum<T>, E> + Send + Sync + 'static,
    ) {
        loop {
            select! {
                maybe_event = subscription.receiver.recv() => {
                    match maybe_event {
                        Ok(event) => {
                            let maybe_datum = match event.as_ref() {
                                SubscriptionEvent::Successful { timestamp, value } => Ok(Datum {
                                    timestamp: *timestamp,
                                    value,
                                }),
                                SubscriptionEvent::Update { timestamp, value } => Ok(Datum {
                                    timestamp: *timestamp,
                                    value,
                                }),
                                SubscriptionEvent::Failure { error } => Err(error),
                            };
                            let maybe_datum = op(maybe_datum);
                            match maybe_datum {
                                Ok(datum) => {
                                    let history = *self.history.lock().await;
                                    self.sender.send_modify(|value| handle_update(value, datum, history))
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

fn handle_update<T, E>(value: &mut Result<Vec<Datum<T>>, E>, datum: Datum<T>, history: Duration) {
    match value {
        Ok(ref mut buffer) => {
            let right = buffer.partition_point(|sample| sample.timestamp < datum.timestamp);
            let left =
                buffer.partition_point(|sample| sample.timestamp < datum.timestamp - history);
            *value = Ok(buffer.drain(left..right).chain(once(datum)).collect());
        }
        Err(_) => {
            *value = Ok(TimeSeries::from([datum]));
        }
    }
}
