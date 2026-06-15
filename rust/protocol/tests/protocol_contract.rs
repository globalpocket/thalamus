use serde_json::{json, Value};
use thalamus_protocol::{deserialize, payload::{RuntimeAgentErrorPayload, RuntimeAgentExitPayload, RuntimeAgentReadyPayload, RuntimeLLMRequestPayload, RuntimeLLMResponsePayload, RuntimeTaskAssignPayload, RuntimeTaskResultPayload, RuntimeToolRequestPayload, RuntimeToolResultPayload}, serialize, subject, EventEnvelope};

#[test]
fn contract_event_envelope_preserves_public_fields_and_serializes_type() {
    let envelope = EventEnvelope::new(
        "event-1".to_string(),
        "runtime.agent.ready".to_string(),
        "subject.created".to_string(),
        "protocol-contract".to_string(),
        "2026-01-01T00:00:00Z".to_string(),
        "subject.v1".to_string(),
        Some("runtime".to_string()),
        vec!["ref-1".to_string(), "ref-2".to_string()],
        json!({ "name": "example", "nested": { "count": 2 } }),
        Some("correlation-1".to_string()),
        Some("causation-1".to_string()),
        json!({ "tenant": "test", "trace": { "sampled": true } }),
    );

    assert_eq!(envelope.id, "event-1");
    assert_eq!(envelope.r#type, "runtime.agent.ready");
    assert_eq!(envelope.subject, "subject.created");
    assert_eq!(envelope.source, "protocol-contract");
    assert_eq!(envelope.timestamp, "2026-01-01T00:00:00Z");
    assert_eq!(envelope.schema, "subject.v1");
    assert_eq!(envelope.scope, Some("runtime".to_string()));
    assert_eq!(envelope.refs, vec!["ref-1".to_string(), "ref-2".to_string()]);
    assert_eq!(envelope.payload, json!({ "name": "example", "nested": { "count": 2 } }));
    assert_eq!(envelope.correlation_id, Some("correlation-1".to_string()));
    assert_eq!(envelope.causation_id, Some("causation-1".to_string()));
    assert_eq!(envelope.metadata, json!({ "tenant": "test", "trace": { "sampled": true } }));

    let serialized = serialize(&envelope).expect("envelope serializes to JSON");
    let serialized_value: Value = serde_json::from_str(&serialized).expect("serialized envelope is JSON");
    let deserialized = deserialize(&serialized).expect("serialized JSON deserializes to envelope");

    assert_eq!(serialized_value["type"], json!("runtime.agent.ready"));
    assert!(serialized_value.get("r#type").is_none());
    assert_eq!(serialized_value["scope"], json!("runtime"));
    assert_eq!(serialized_value["refs"], json!(["ref-1", "ref-2"]));
    assert_eq!(serialized_value["payload"], json!({ "name": "example", "nested": { "count": 2 } }));
    assert_eq!(serialized_value["correlation_id"], json!("correlation-1"));
    assert_eq!(serialized_value["causation_id"], json!("causation-1"));
    assert_eq!(serialized_value["metadata"], json!({ "tenant": "test", "trace": { "sampled": true } }));
    assert_eq!(deserialized.scope, Some("runtime".to_string()));
    assert_eq!(deserialized.refs, vec!["ref-1".to_string(), "ref-2".to_string()]);
    assert_eq!(deserialized.payload, json!({ "name": "example", "nested": { "count": 2 } }));
    assert_eq!(deserialized.correlation_id, Some("correlation-1".to_string()));
    assert_eq!(deserialized.causation_id, Some("causation-1".to_string()));
    assert_eq!(deserialized.metadata, json!({ "tenant": "test", "trace": { "sampled": true } }));
    assert_eq!(deserialized, envelope);
}

#[test]
fn contract_runtime_subject_constants_match_canonical_values() {
    assert_eq!(subject::RUNTIME_AGENT_READY, "runtime.agent.ready");
    assert_eq!(subject::RUNTIME_AGENT_EXIT, "runtime.agent.exit");
    assert_eq!(subject::RUNTIME_AGENT_ERROR, "runtime.agent.error");
    assert_eq!(subject::RUNTIME_TASK_ASSIGN, "runtime.task.assign");
    assert_eq!(subject::RUNTIME_TASK_ASSIGN_AGENT_TEMPLATE, "runtime.task.assign.<agent_id>");
    assert_eq!(subject::RUNTIME_TASK_RESULT, "runtime.task.result");
    assert_eq!(subject::RUNTIME_TOOL_REQUEST, "runtime.tool.request");
    assert_eq!(subject::RUNTIME_TOOL_RESULT, "runtime.tool.result");
    assert_eq!(subject::RUNTIME_LLM_REQUEST, "runtime.llm.request");
    assert_eq!(subject::RUNTIME_LLM_RESPONSE, "runtime.llm.response");
}

#[test]
fn contract_runtime_task_assign_for_agent_generates_dynamic_subject() {
    assert_eq!(
        subject::runtime_task_assign_for_agent("agent-1"),
        "runtime.task.assign.agent-1"
    );
}

#[test]
fn contract_runtime_payload_structs_round_trip_public_fields() {
    let task_assign = RuntimeTaskAssignPayload {
        task_id: "task-1".to_string(),
        agent_id: "agent-1".to_string(),
        input: json!({ "prompt": "run" }),
        capabilities: vec!["shell".to_string(), "llm".to_string()],
        metadata: json!({ "priority": "high" }),
    };
    let task_assign_json = serde_json::to_value(&task_assign).expect("task assign payload serializes");
    let task_assign_round_trip: RuntimeTaskAssignPayload = serde_json::from_value(task_assign_json).expect("task assign payload deserializes");
    assert_eq!(task_assign_round_trip, task_assign);
    assert_eq!(task_assign_round_trip.capabilities, vec!["shell".to_string(), "llm".to_string()]);

    let task_result = RuntimeTaskResultPayload {
        task_id: "task-1".to_string(),
        status: "completed".to_string(),
        summary: Some("done".to_string()),
        result: Some(json!({ "ok": true })),
    };
    let task_result_json = serde_json::to_value(&task_result).expect("task result payload serializes");
    let task_result_round_trip: RuntimeTaskResultPayload = serde_json::from_value(task_result_json).expect("task result payload deserializes");
    assert_eq!(task_result_round_trip, task_result);

    let agent_ready = RuntimeAgentReadyPayload {
        agent_id: "agent-1".to_string(),
        capabilities: vec!["shell".to_string()],
    };
    let agent_ready_json = serde_json::to_value(&agent_ready).expect("agent ready payload serializes");
    let agent_ready_round_trip: RuntimeAgentReadyPayload = serde_json::from_value(agent_ready_json).expect("agent ready payload deserializes");
    assert_eq!(agent_ready_round_trip, agent_ready);

    let agent_exit = RuntimeAgentExitPayload {
        agent_id: "agent-1".to_string(),
        reason: Some("shutdown".to_string()),
    };
    let agent_exit_json = serde_json::to_value(&agent_exit).expect("agent exit payload serializes");
    let agent_exit_round_trip: RuntimeAgentExitPayload = serde_json::from_value(agent_exit_json).expect("agent exit payload deserializes");
    assert_eq!(agent_exit_round_trip, agent_exit);

    let tool_request = RuntimeToolRequestPayload {
        request_id: "request-1".to_string(),
        task_id: Some("task-1".to_string()),
        capability: "shell".to_string(),
        input: json!({ "cmd": "echo ok" }),
        agent_id: Some("agent-1".to_string()),
    };
    let tool_request_json = serde_json::to_value(&tool_request).expect("tool request payload serializes");
    let tool_request_round_trip: RuntimeToolRequestPayload = serde_json::from_value(tool_request_json).expect("tool request payload deserializes");
    assert_eq!(tool_request_round_trip, tool_request);

    let tool_result = RuntimeToolResultPayload {
        request_id: "request-1".to_string(),
        task_id: "task-1".to_string(),
        status: "completed".to_string(),
        output: Some(json!({ "stdout": "ok" })),
        error: None,
    };
    let tool_result_json = serde_json::to_value(&tool_result).expect("tool result payload serializes");
    let tool_result_round_trip: RuntimeToolResultPayload = serde_json::from_value(tool_result_json).expect("tool result payload deserializes");
    assert_eq!(tool_result_round_trip, tool_result);

    let llm_request = RuntimeLLMRequestPayload {
        request_id: "request-2".to_string(),
        task_id: Some("task-2".to_string()),
        prompt: "summarize".to_string(),
        model: Some("model-a".to_string()),
        agent_id: Some("agent-2".to_string()),
    };
    let llm_request_json = serde_json::to_value(&llm_request).expect("llm request payload serializes");
    let llm_request_round_trip: RuntimeLLMRequestPayload = serde_json::from_value(llm_request_json).expect("llm request payload deserializes");
    assert_eq!(llm_request_round_trip, llm_request);

    let llm_response = RuntimeLLMResponsePayload {
        request_id: "request-2".to_string(),
        task_id: "task-2".to_string(),
        status: "completed".to_string(),
        text: Some("summary".to_string()),
        model: "model-a".to_string(),
        error: None,
    };
    let llm_response_json = serde_json::to_value(&llm_response).expect("llm response payload serializes");
    let llm_response_round_trip: RuntimeLLMResponsePayload = serde_json::from_value(llm_response_json).expect("llm response payload deserializes");
    assert_eq!(llm_response_round_trip, llm_response);

    let agent_error = RuntimeAgentErrorPayload {
        agent_id: Some("agent-3".to_string()),
        error: "failed".to_string(),
        task_id: Some("task-3".to_string()),
    };
    let agent_error_json = serde_json::to_value(&agent_error).expect("agent error payload serializes");
    let agent_error_round_trip: RuntimeAgentErrorPayload = serde_json::from_value(agent_error_json).expect("agent error payload deserializes");
    assert_eq!(agent_error_round_trip, agent_error);
}
