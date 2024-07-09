use std::time::SystemTime;

use bincode::{DefaultOptions, Deserializer, Options};
use path_serde::PathDeserialize;
use serde::Deserialize;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::messages::{Path, TextOrBinary};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] path_serde::deserialize::Error<serde_json::Error>),
    #[error(transparent)]
    Bincode(#[from] path_serde::deserialize::Error<bincode::Error>),
}

pub enum Event {
    Write {
        path: Path,
        timestamp: SystemTime,
        value: TextOrBinary,
        return_sender: oneshot::Sender<Result<(), Error>>,
    },
}

pub struct SinkHandle {
    command_sender: mpsc::Sender<Event>,
}

impl SinkHandle {
    pub async fn write(
        &self,
        path: impl Into<Path>,
        timestamp: SystemTime,
        value: TextOrBinary,
    ) -> Result<(), Error> {
        let (return_sender, return_receiver) = oneshot::channel();
        self.command_sender
            .send(Event::Write {
                path: path.into(),
                timestamp,
                value,
                return_sender,
            })
            .await
            .unwrap();
        return_receiver.await.unwrap()
    }
}

pub struct Sink<T> {
    data_sender: buffered_watch::Sender<(SystemTime, T)>,
    command_receiver: mpsc::Receiver<Event>,
}

impl<T> Sink<T>
where
    for<'de> T: Deserialize<'de> + PathDeserialize + Clone,
{
    pub fn new(data_sender: buffered_watch::Sender<(SystemTime, T)>) -> (Self, SinkHandle) {
        let (command_sender, command_receiver) = mpsc::channel(1);
        let task = Self {
            data_sender,
            command_receiver,
        };
        let handle = SinkHandle { command_sender };
        (task, handle)
    }

    pub async fn run(mut self) {
        while let Some(command) = self.command_receiver.recv().await {
            match command {
                Event::Write {
                    path,
                    timestamp,
                    value,
                    return_sender,
                } => {
                    let result = self.write(&path, timestamp, value);
                    let _ = return_sender.send(result);
                }
            }
        }
    }

    fn write(
        &mut self,
        path: &str,
        timestamp: SystemTime,
        value: TextOrBinary,
    ) -> Result<(), Error> {
        let data = if path.is_empty() {
            match value {
                TextOrBinary::Text(text) => serde_json::from_value(text)
                    .map_err(path_serde::deserialize::Error::DeserializationFailed)
                    .map_err(Error::Json)?,
                TextOrBinary::Binary(bytes) => bincode::deserialize(&bytes)
                    .map_err(path_serde::deserialize::Error::DeserializationFailed)
                    .map_err(Error::Bincode)?,
            }
        } else {
            let mut data = self.data_sender.borrow().1.clone();

            match value {
                TextOrBinary::Text(text) => {
                    data.deserialize_path(path, text).map_err(Error::Json)?;
                }
                TextOrBinary::Binary(bytes) => {
                    data.deserialize_path(
                        path,
                        &mut Deserializer::from_slice(
                            &bytes,
                            DefaultOptions::new()
                                .with_fixint_encoding()
                                .allow_trailing_bytes(),
                        ),
                    )
                    .map_err(Error::Bincode)?;
                }
            };
            data
        };

        *self.data_sender.borrow_mut() = (timestamp, data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use path_serde::PathDeserialize;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize, PathDeserialize, Clone)]
    struct Data {
        foo: usize,
    }

    #[tokio::test]
    async fn write_text() {
        let (data_sender, mut data_receiver) =
            buffered_watch::channel((UNIX_EPOCH, Data { foo: 42 }));
        let (sink, handle) = Sink::new(data_sender);
        let task = tokio::spawn(sink.run());

        let path = Path::from("foo");
        let value = TextOrBinary::Text(json!(1337));
        let now = SystemTime::now();
        handle.write(path, now, value).await.unwrap();

        drop(handle);
        task.await.unwrap();

        let (timestamp, data) = &*data_receiver.borrow();
        assert_eq!(timestamp, &now);
        assert_eq!(data.foo, 1337);
    }

    #[tokio::test]
    async fn write_binary() {
        let (data_sender, mut data_receiver) =
            buffered_watch::channel((UNIX_EPOCH, Data { foo: 42 }));
        let (sink, handle) = Sink::new(data_sender);
        let task = tokio::spawn(sink.run());

        let path = Path::from("foo");
        let number: usize = 1337;
        let value = TextOrBinary::Binary(bincode::serialize(&number).unwrap());
        let now = SystemTime::now();
        handle.write(path, now, value).await.unwrap();

        drop(handle);
        task.await.unwrap();

        let (timestamp, data) = &*data_receiver.borrow();
        assert_eq!(timestamp, &now);
        assert_eq!(data.foo, 1337);
    }

    #[tokio::test]
    async fn write_invalid() {
        let (data_sender, _data_receiver) = buffered_watch::channel((UNIX_EPOCH, Data { foo: 42 }));
        let (sink, handle) = Sink::new(data_sender);
        let task = tokio::spawn(sink.run());

        let path = Path::from("foo");
        let value = TextOrBinary::Text(json!("invalid"));
        let now = SystemTime::now();
        let result = handle.write(path, now, value).await;

        assert!(result.is_err());

        drop(handle);
        task.await.unwrap();
    }
}
