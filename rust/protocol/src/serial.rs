use crate::{message::EventEnvelope, ProtocolError};

pub fn serialize(envelope: &EventEnvelope) -> Result<String, ProtocolError> {
    serde_json::to_string(envelope).map_err(|e| ProtocolError::SerializationError(e.to_string()))
}

pub fn deserialize(s: &str) -> Result<EventEnvelope, ProtocolError> {
    serde_json::from_str(s).map_err(|e| ProtocolError::DeserializationError(e.to_string()))
}
