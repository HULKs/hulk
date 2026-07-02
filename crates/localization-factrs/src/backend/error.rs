use thiserror::Error;

#[derive(Debug, Error)]
pub enum VinsBackendError {
    #[error("frontend disconnected")]
    FrontendDisconnected,
}
