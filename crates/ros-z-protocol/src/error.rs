use crate::entity::EntityConversionError;

pub type Result<T> = std::result::Result<T, ProtocolError>;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("invalid key expression '{expression}'")]
    InvalidKeyExpression {
        expression: String,
        #[source]
        source: zenoh::Error,
    },

    #[error("failed to parse ros-z liveliness key '{key_expr}'")]
    ParseLiveliness {
        key_expr: String,
        #[source]
        source: EntityConversionError,
    },
}
