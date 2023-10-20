use log::error;
use tokio::sync::mpsc;

pub async fn notify_all(notification_senders: &[mpsc::Sender<()>]) {
    for sender in notification_senders {
        if let Err(error) = sender.send(()).await {
            error!("{error:?}");
        };
    }
}
