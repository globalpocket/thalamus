pub mod message;
pub mod serial;

pub use message::EventEnvelope;
pub use serial::{serialize, deserialize};

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("serialization error: {0}")]
    SerializationError(String),
    #[error("deserialization error: {0}")]
    DeserializationError(String),
    #[error("invalid envelope: {0}")]
    InvalidEnvelope(String),
}
