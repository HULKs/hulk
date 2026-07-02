use std::{future::Future, time::Duration};

use color_eyre::Result;
use tokio::sync::watch;

const RETRY_DELAY: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryCommand<T> {
    pub target: T,
    pub timeout: Duration,
}

pub async fn run_retrying_rpc_worker<T, Call, Fut>(
    mut commands: watch::Receiver<Option<RetryCommand<T>>>,
    mut call: Call,
) where
    T: Copy + Send + Sync + 'static,
    Call: FnMut(RetryCommand<T>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send,
{
    loop {
        if commands.changed().await.is_err() {
            break;
        }

        loop {
            let Some(command) = *commands.borrow_and_update() else {
                break;
            };
            if call(command).await.is_ok() {
                break;
            }
            if wait_before_retry(&mut commands).await.is_err() {
                break;
            }
        }
    }
}

async fn wait_before_retry<T>(commands: &mut watch::Receiver<Option<RetryCommand<T>>>) -> Result<()>
where
    T: Copy + Send + Sync + 'static,
{
    tokio::select! {
        result = commands.changed() => {
            result?;
        }
        _ = tokio::time::sleep(RETRY_DELAY) => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retries_same_target_after_failure() {
        let (command_sender, command_receiver) = watch::channel(None);
        let (call_sender, mut calls) = tokio::sync::mpsc::unbounded_channel();
        let worker = tokio::spawn(run_retrying_rpc_worker(
            command_receiver,
            move |command: RetryCommand<bool>| {
                let call_sender = call_sender.clone();
                async move {
                    let (completion_sender, completion_receiver) = tokio::sync::oneshot::channel();
                    call_sender
                        .send((command.target, completion_sender))
                        .unwrap();
                    completion_receiver.await.unwrap()
                }
            },
        ));

        command_sender
            .send(Some(RetryCommand {
                target: true,
                timeout: Duration::from_millis(10),
            }))
            .unwrap();
        let (first_target, first_completion) = calls.recv().await.unwrap();
        assert!(first_target);

        first_completion
            .send(Err(color_eyre::eyre::eyre!("failed")))
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), calls.recv())
                .await
                .is_err()
        );
        let (second_target, second_completion) = calls.recv().await.unwrap();
        assert!(second_target);

        second_completion.send(Ok(())).unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(10), calls.recv())
                .await
                .is_err()
        );
        worker.abort();
    }

    #[tokio::test]
    async fn sends_latest_target_after_stale_success() {
        let (command_sender, command_receiver) = watch::channel(None);
        let (call_sender, mut calls) = tokio::sync::mpsc::unbounded_channel();
        let worker = tokio::spawn(run_retrying_rpc_worker(
            command_receiver,
            move |command: RetryCommand<bool>| {
                let call_sender = call_sender.clone();
                async move {
                    let (completion_sender, completion_receiver) = tokio::sync::oneshot::channel();
                    call_sender
                        .send((command.target, completion_sender))
                        .unwrap();
                    completion_receiver.await.unwrap()
                }
            },
        ));

        command_sender
            .send(Some(RetryCommand {
                target: true,
                timeout: Duration::from_millis(10),
            }))
            .unwrap();
        let (first_target, first_completion) = calls.recv().await.unwrap();
        assert!(first_target);

        command_sender
            .send(Some(RetryCommand {
                target: false,
                timeout: Duration::from_millis(10),
            }))
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(10), calls.recv())
                .await
                .is_err()
        );
        first_completion.send(Ok(())).unwrap();
        let (second_target, second_completion) = calls.recv().await.unwrap();
        assert!(!second_target);

        second_completion.send(Ok(())).unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(10), calls.recv())
                .await
                .is_err()
        );
        worker.abort();
    }

    #[tokio::test]
    async fn sends_latest_target_after_stale_failure() {
        let (command_sender, command_receiver) = watch::channel(None);
        let (call_sender, mut calls) = tokio::sync::mpsc::unbounded_channel();
        let worker = tokio::spawn(run_retrying_rpc_worker(
            command_receiver,
            move |command: RetryCommand<bool>| {
                let call_sender = call_sender.clone();
                async move {
                    let (completion_sender, completion_receiver) = tokio::sync::oneshot::channel();
                    call_sender
                        .send((command.target, completion_sender))
                        .unwrap();
                    completion_receiver.await.unwrap()
                }
            },
        ));

        command_sender
            .send(Some(RetryCommand {
                target: true,
                timeout: Duration::from_millis(10),
            }))
            .unwrap();
        let (first_target, first_completion) = calls.recv().await.unwrap();
        assert!(first_target);

        command_sender
            .send(Some(RetryCommand {
                target: false,
                timeout: Duration::from_millis(10),
            }))
            .unwrap();
        first_completion
            .send(Err(color_eyre::eyre::eyre!("failed")))
            .unwrap();
        let (second_target, second_completion) = calls.recv().await.unwrap();
        assert!(!second_target);

        second_completion.send(Ok(())).unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(10), calls.recv())
                .await
                .is_err()
        );
        worker.abort();
    }
}
