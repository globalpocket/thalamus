use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventEnvelope {
    pub id: String,
    pub subject: String,
    pub source: String,
    pub timestamp: String,
    pub schema: String,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub metadata: serde_json::Value,
}

impl EventEnvelope {
    pub fn new(
        id: String,
        subject: String,
        source: String,
        timestamp: String,
        schema: String,
        payload: serde_json::Value,
        correlation_id: Option<String>,
        causation_id: Option<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            subject,
            source,
            timestamp,
            schema,
            payload,
            correlation_id,
            causation_id,
            metadata,
        }
    }
}
