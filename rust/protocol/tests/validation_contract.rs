use thalamus_protocol::{
    payload::{
        RuntimeAgentErrorPayload, RuntimeAgentExitPayload, RuntimeAgentReadyPayload,
        RuntimeAgentSpawnPayload, RuntimeLLMRequestPayload, RuntimeTaskAssignPayload,
        RuntimeTaskResultPayload, RuntimeToolRequestPayload,
    },
    validation::validate_and_normalize_payload,
};

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
fn valid_agent_spawn_is_normalized() {
    let p = serde_json::json!({"state": "starting"});
    let result = validate_and_normalize_payload("runtime.agent.spawn", "evt-1", p.clone());
    assert!(result.is_ok());
    let normalized = result.unwrap();
    assert_eq!(normalized["state"], "starting");
}

#[test]
fn invalid_agent_spawn_rejects_missing_state() {
    let p = serde_json::json!({});
    let result = validate_and_normalize_payload("runtime.agent.spawn", "evt-1", p.clone());
    assert!(result.is_err());
}

#[test]
fn valid_agent_exit_is_normalized() {
    let p = serde_json::json!({
        "agent_id": "agent-1",
        "reason": "shutdown"
    });
    let result = validate_and_normalize_payload("runtime.agent.exit", "evt-1", p.clone());
    assert!(result.is_ok());
}

#[test]
fn invalid_agent_exit_rejects_missing_agent_id() {
    let p = serde_json::json!({"reason": "shutdown"});
    let result = validate_and_normalize_payload("runtime.agent.exit", "evt-1", p.clone());
    assert!(result.is_err());
}

#[test]
fn valid_agent_error_is_normalized() {
    let p = serde_json::json!({
        "agent_id": "agent-1",
        "error": {"message": "fail"}
    });
    let result = validate_and_normalize_payload("runtime.agent.error", "evt-1", p.clone());
    assert!(result.is_ok());
}

#[test]
fn valid_task_assign_is_normalized() {
    let p = serde_json::json!({
        "task_id": "task-1",
        "input": {"key": "value"},
        "capabilities": ["llm"]
    });
    let result = validate_and_normalize_payload("runtime.task.assign", "evt-1", p.clone());
    assert!(result.is_ok());
}

#[test]
fn invalid_task_assign_rejects_missing_task_id() {
    let p = serde_json::json!({"input": {}});
    let result = validate_and_normalize_payload("runtime.task.assign", "evt-1", p.clone());
    assert!(result.is_err());
}

#[test]
fn valid_task_result_is_normalized() {
    let p = serde_json::json!({
        "task_id": "task-1",
        "status": "completed"
    });
    let result = validate_and_normalize_payload("runtime.task.result", "evt-1", p.clone());
    assert!(result.is_ok());
}

#[test]
fn invalid_task_result_rejects_missing_task_id() {
    let p = serde_json::json!({"status": "completed"});
    let result = validate_and_normalize_payload("runtime.task.result", "evt-1", p.clone());
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

#[test]
fn invalid_canonical_payload_is_not_recorded() {
    // Invalid canonical payload should return error and not be published
    let p = serde_json::json!({"invalid": "payload"});
    let result = validate_and_normalize_payload("runtime.agent.ready", "evt-1", p.clone());
    assert!(result.is_err());
}

#[test]
fn unknown_extension_subject_is_recorded() {
    // Unknown extension subjects should pass validation
    let p = serde_json::json!({"any": "data"});
    let result = validate_and_normalize_payload("custom.extension", "evt-1", p.clone());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), p);
}

#[test]
fn request_id_added_before_recording() {
    // request_id should be auto-completed for request subjects
    let p = serde_json::json!({
        "task_id": "task-1",
        "prompt": "hello"
    });
    let result = validate_and_normalize_payload("runtime.llm.request", "evt-req-1", p.clone());
    assert!(result.is_ok());
    let normalized = result.unwrap();
    assert_eq!(normalized["request_id"], "evt-req-1");
}
