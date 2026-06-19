pub mod message;
pub mod payload;
pub mod serial;
pub mod subject;
pub mod validation;

pub use message::EventEnvelope;
pub use serial::{deserialize, serialize};
pub use validation::{validate_and_normalize_payload, ValidationError};

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("serialization error: {0}")]
    SerializationError(String),
    #[error("deserialization error: {0}")]
    DeserializationError(String),
    #[error("invalid envelope: {0}")]
    InvalidEnvelope(String),
}
