use crate::entity::EntityConversionError;

pub type Result<T> = std::result::Result<T, ProtocolError>;

/// Errors produced while generating or parsing ros-z protocol key expressions.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// Generated key expression was rejected by Zenoh.
    #[error("invalid key expression '{expression}'")]
    InvalidKeyExpression {
        expression: String,
        #[source]
        source: zenoh::Error,
    },

    /// Liveliness key expression could not be converted into a ros-z entity.
    #[error("failed to parse ros-z liveliness key '{key_expr}'")]
    ParseLiveliness {
        key_expr: String,
        #[source]
        source: EntityConversionError,
    },
}
