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
    pub scope: Option<String>,
    pub refs: Vec<String>,
    pub payload: Value,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub metadata: Value,
    // LCOV_EXCL_STOP
}

impl EventEnvelope {
    pub fn new(
        id: String,
        r#type: String,
        subject: String,
        source: String,
        timestamp: String,
        schema: String,
        // LCOV_EXCL_START
        scope: Option<String>,
        refs: Vec<String>,
        payload: Value,
        correlation_id: Option<String>,
        causation_id: Option<String>,
        metadata: Value,
        // LCOV_EXCL_STOP
    ) -> Self {
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
