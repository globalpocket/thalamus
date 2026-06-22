use serde_json::Value;

use crate::payload::{
    RuntimeAgentErrorPayload, RuntimeAgentExitPayload, RuntimeAgentReadyPayload,
    RuntimeLLMRequestPayload, RuntimeLLMResponsePayload, RuntimeTaskAssignPayload,
    RuntimeTaskResultPayload, RuntimeToolRequestPayload, RuntimeToolResultPayload,
};

/// Validation error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub subject: String,
    pub reason: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "validation error for subject '{}': {}",
            self.subject, self.reason
        )
    }
}

impl std::error::Error for ValidationError {}

/// Canonical subjects that require payload validation
const CANONICAL_SUBJECTS: &[&str] = &[
    "runtime.agent.spawn",
    "runtime.agent.ready",
    "runtime.agent.exit",
    "runtime.agent.error",
    "runtime.task.assign",
    "runtime.task.result",
    "runtime.llm.request",
    "runtime.llm.response",
    "runtime.tool.request",
    "runtime.tool.result",
];

/// Validate and normalize a payload for a given subject.
///
/// - For canonical subjects, deserializes to the corresponding struct to validate,
///   then re-serializes to normalize.
/// - For unknown/extension subjects, returns the payload unchanged.
/// - Returns ValidationError for invalid canonical payloads.
pub fn validate_and_normalize_payload(
    subject: &str,
    event_id: &str,
    payload: Value,
) -> Result<Value, ValidationError> {
    if !is_canonical_subject(subject) {
        return Ok(payload);
    }

    let normalized = match subject {
        "runtime.agent.spawn" => {
            // spawn has no strict payload struct in MVP; accept as-is
            payload
        }
        "runtime.agent.ready" => {
            let p: RuntimeAgentReadyPayload =
                serde_json::from_value(payload).map_err(|e| ValidationError {
                    subject: subject.to_string(),
                    reason: format!("RuntimeAgentReadyPayload: {}", e),
                })?;
            serde_json::to_value(&p).map_err(|e| ValidationError {
                subject: subject.to_string(),
                reason: format!("serialize RuntimeAgentReadyPayload: {}", e),
            })?
        }
        "runtime.agent.exit" => {
            let p: RuntimeAgentExitPayload =
                serde_json::from_value(payload).map_err(|e| ValidationError {
                    subject: subject.to_string(),
                    reason: format!("RuntimeAgentExitPayload: {}", e),
                })?;
            serde_json::to_value(&p).map_err(|e| ValidationError {
                subject: subject.to_string(),
                reason: format!("serialize RuntimeAgentExitPayload: {}", e),
            })?
        }
        "runtime.agent.error" => {
            let p: RuntimeAgentErrorPayload =
                serde_json::from_value(payload).map_err(|e| ValidationError {
                    subject: subject.to_string(),
                    reason: format!("RuntimeAgentErrorPayload: {}", e),
                })?;
            serde_json::to_value(&p).map_err(|e| ValidationError {
                subject: subject.to_string(),
                reason: format!("serialize RuntimeAgentErrorPayload: {}", e),
            })?
        }
        "runtime.task.assign" => {
            let p: RuntimeTaskAssignPayload =
                serde_json::from_value(payload).map_err(|e| ValidationError {
                    subject: subject.to_string(),
                    reason: format!("RuntimeTaskAssignPayload: {}", e),
                })?;
            serde_json::to_value(&p).map_err(|e| ValidationError {
                subject: subject.to_string(),
                reason: format!("serialize RuntimeTaskAssignPayload: {}", e),
            })?
        }
        "runtime.task.result" => {
            let p: RuntimeTaskResultPayload =
                serde_json::from_value(payload).map_err(|e| ValidationError {
                    subject: subject.to_string(),
                    reason: format!("RuntimeTaskResultPayload: {}", e),
                })?;
            serde_json::to_value(&p).map_err(|e| ValidationError {
                subject: subject.to_string(),
                reason: format!("serialize RuntimeTaskResultPayload: {}", e),
            })?
        }
        "runtime.llm.request" => {
            let p: RuntimeLLMRequestPayload =
                serde_json::from_value(payload).map_err(|e| ValidationError {
                    subject: subject.to_string(),
                    reason: format!("RuntimeLLMRequestPayload: {}", e),
                })?;
            serde_json::to_value(&p).map_err(|e| ValidationError {
                subject: subject.to_string(),
                reason: format!("serialize RuntimeLLMRequestPayload: {}", e),
            })?
        }
        "runtime.llm.response" => {
            let p: RuntimeLLMResponsePayload =
                serde_json::from_value(payload).map_err(|e| ValidationError {
                    subject: subject.to_string(),
                    reason: format!("RuntimeLLMResponsePayload: {}", e),
                })?;
            serde_json::to_value(&p).map_err(|e| ValidationError {
                subject: subject.to_string(),
                reason: format!("serialize RuntimeLLMResponsePayload: {}", e),
            })?
        }
        "runtime.tool.request" => {
            let p: RuntimeToolRequestPayload =
                serde_json::from_value(payload).map_err(|e| ValidationError {
                    subject: subject.to_string(),
                    reason: format!("RuntimeToolRequestPayload: {}", e),
                })?;
            serde_json::to_value(&p).map_err(|e| ValidationError {
                subject: subject.to_string(),
                reason: format!("serialize RuntimeToolRequestPayload: {}", e),
            })?
        }
        "runtime.tool.result" => {
            let p: RuntimeToolResultPayload =
                serde_json::from_value(payload).map_err(|e| ValidationError {
                    subject: subject.to_string(),
                    reason: format!("RuntimeToolResultPayload: {}", e),
                })?;
            serde_json::to_value(&p).map_err(|e| ValidationError {
                subject: subject.to_string(),
                reason: format!("serialize RuntimeToolResultPayload: {}", e),
            })?
        }
        _ => payload,
    };

    // For request subjects, 補完 request_id if None
    let normalized = match subject {
        "runtime.llm.request" | "runtime.tool.request" => {
            if normalized
                .get("request_id")
                .and_then(|v| v.as_str())
                .is_none()
            {
                let mut n = normalized;
                n["request_id"] = Value::String(event_id.to_string());
                n
            } else {
                normalized
            }
        }
        _ => normalized,
    };

    Ok(normalized)
}

fn is_canonical_subject(subject: &str) -> bool {
    CANONICAL_SUBJECTS.contains(&subject)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_subject_returns_payload_unchanged() {
        let p = serde_json::json!({"custom": "data"});
        let result = validate_and_normalize_payload("custom.extension", "evt-1", p.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), p);
    }

    #[test]
    fn valid_agent_ready_is_normalized() {
        let p = serde_json::json!({
            "agent_id": "agent-1",
            "capabilities": ["cap.a"]
        });
        let result = validate_and_normalize_payload("runtime.agent.ready", "evt-1", p.clone());
        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert_eq!(normalized["agent_id"], "agent-1");
    }

    #[test]
    fn invalid_agent_ready_rejects_missing_agent_id() {
        let p = serde_json::json!({"capabilities": ["cap.a"]});
        let result = validate_and_normalize_payload("runtime.agent.ready", "evt-1", p.clone());
        assert!(result.is_err());
    }

    #[test]
    fn llm_request_without_request_id_gets_event_id() {
        let p = serde_json::json!({
            "task_id": "task-1",
            "prompt": "hello"
        });
        let result = validate_and_normalize_payload("runtime.llm.request", "evt-123", p.clone());
        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert_eq!(normalized["request_id"], "evt-123");
    }

    #[test]
    fn llm_request_with_explicit_request_id_preserves_it() {
        let p = serde_json::json!({
            "task_id": "task-1",
            "request_id": "custom-req-id",
            "prompt": "hello"
        });
        let result = validate_and_normalize_payload("runtime.llm.request", "evt-1", p.clone());
        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert_eq!(normalized["request_id"], "custom-req-id");
    }

    #[test]
    fn tool_request_without_request_id_gets_event_id() {
        let p = serde_json::json!({
            "task_id": "task-1",
            "capability": "tool.echo",
            "input": {"key": "value"}
        });
        let result = validate_and_normalize_payload("runtime.tool.request", "evt-456", p.clone());
        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert_eq!(normalized["request_id"], "evt-456");
    }

    #[test]
    fn invalid_llm_request_rejects_missing_task_id() {
        let p = serde_json::json!({"prompt": "hello"});
        let result = validate_and_normalize_payload("runtime.llm.request", "evt-1", p.clone());
        assert!(result.is_err());
    }

    #[test]
    fn invalid_tool_request_rejects_missing_capability() {
        let p = serde_json::json!({"task_id": "task-1", "input": {}});
        let result = validate_and_normalize_payload("runtime.tool.request", "evt-1", p.clone());
        assert!(result.is_err());
    }
}
