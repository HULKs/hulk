use std::future::pending;

use awaitgroup::WaitGroup;
use log::{error, info};
use tokio::{net::TcpListener, select, spawn, sync::mpsc::Sender, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::framework::{communication::connection::connection, Configuration};

use super::{database_subscription_manager, parameter_modificator};

pub async fn acceptor(
    initial_configuration: Configuration,
    database_subscription_manager_sender: Sender<database_subscription_manager::Request>,
    parameter_modificator_sender: Sender<parameter_modificator::Request>,
    keep_running: CancellationToken,
) -> JoinHandle<()> {
    spawn(async move {
        if initial_configuration.disable_communication_acceptor {
            keep_running.cancelled().await;
            return;
        }

        let mut wait_group = WaitGroup::new();
        select! {
            _ = async {
                let listener = match TcpListener::bind("0.0.0.0:1337").await {
                    Ok(listener) => listener,
                    Err(error) => {
                        error!("Failed to listen: {:?}", error);
                        keep_running.cancel();
                        pending().await
                    },
                };
                loop {
                    let stream = match listener
                        .accept()
                        .await {
                            Ok((stream, _)) => stream,
                            Err(error) => {
                                error!("Failed to accept connection: {:?}", error);
                                keep_running.cancel();
                                pending().await
                            }
                        };
                    info!("New connection: {:?}", stream);
                    match connection(stream, database_subscription_manager_sender.clone(), parameter_modificator_sender.clone(), keep_running.clone(), wait_group.worker()).await {
                        Ok(_) => {},
                        Err(error) => error!("Failed to establish connection: {:?}", error),
                    }
                }
            } => {},
            _ = keep_running.cancelled() => {},
        };

        wait_group.wait().await;
    })
}
