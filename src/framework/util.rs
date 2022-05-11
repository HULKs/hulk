use std::collections::HashSet;

use log::error;
use tokio::sync::broadcast::Receiver;

pub fn collect_changed_parameters(
    receiver: &mut Receiver<String>,
) -> anyhow::Result<HashSet<String>> {
    let mut changed_parameters = HashSet::new();
    loop {
        let value = receiver.try_recv();
        match value {
            Ok(parameter) => {
                changed_parameters.insert(parameter);
            }
            Err(error) => match error {
                tokio::sync::broadcast::error::TryRecvError::Empty => {
                    return Ok(changed_parameters)
                }
                tokio::sync::broadcast::error::TryRecvError::Closed => {
                    anyhow::bail!("Broadcast channel was closed")
                }
                tokio::sync::broadcast::error::TryRecvError::Lagged(number_of_skipped_messages) => {
                    error!(
                    "Changed parameters receiver was lagging and fell behind: Dropped {} messages",
                    number_of_skipped_messages
                )
                }
            },
        }
    }
}
