use core::fmt::Debug;

use futures_util::{Sink, SinkExt};
use log::error;

/// Extension trait for sending messages to a sink.
/// If an error occurs while sending the message, it is logged to the error log.
///
/// # Example
///
/// ```
/// socket.send_or_log(Message::Close(None)).await;
/// ```
pub trait SendOrLogExt<Item> {
    /// Sends a message to the sink and logs any errors that occur.
    async fn send_or_log(&mut self, message: Item);
}

impl<Item, T> SendOrLogExt<Item> for T
where
    Item: Send,
    T: SinkExt<Item> + Unpin + Send,
    <T as Sink<Item>>::Error: Debug,
{
    async fn send_or_log(&mut self, message: Item) {
        if let Err(error) = self.send(message).await {
            error!("failed to send message: {error:#?}");
        }
    }
}
