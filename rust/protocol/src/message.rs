use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventEnvelope {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub subject: String,
    pub source: String,
    pub timestamp: String,
    pub schema: String,
    // LCOV_EXCL_START
    #[serde(default)]
    pub scope: Option<Value>,
    #[serde(default)]
    pub refs: Option<Value>,
    pub payload: Value,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub metadata: Value,
    // LCOV_EXCL_STOP
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventEnvelopeFields {
    pub id: String,
    pub r#type: String,
    pub subject: String,
    pub source: String,
    pub timestamp: String,
    pub schema: String,
    // LCOV_EXCL_START
    pub scope: Option<Value>,
    pub refs: Option<Value>,
    pub payload: Value,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub metadata: Value,
    // LCOV_EXCL_STOP
}

impl EventEnvelope {
    pub fn new(fields: EventEnvelopeFields) -> Self {
        let EventEnvelopeFields {
            id,
            r#type,
            subject,
            source,
            timestamp,
            schema,
            // LCOV_EXCL_START
            scope,
            refs,
            payload,
            correlation_id,
            causation_id,
            metadata,
            // LCOV_EXCL_STOP
        } = fields;

        Self {
            id,
            r#type,
            subject,
            source,
            timestamp,
            schema,
            scope,
            refs,
            payload,
            correlation_id,
            causation_id,
            metadata,
        }
    }
}
