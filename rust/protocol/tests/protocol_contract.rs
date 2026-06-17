use serde_json::{json, Value};
use thalamus_protocol::{
    deserialize,
    message::EventEnvelopeFields,
    payload::{
        RuntimeAgentErrorPayload, RuntimeAgentExitPayload, RuntimeAgentReadyPayload,
        RuntimeAgentSpawnPayload, RuntimeLLMRequestPayload, RuntimeLLMResponsePayload,
        RuntimeTaskAssignPayload, RuntimeTaskResultPayload, RuntimeToolRequestPayload,
        RuntimeToolResultPayload,
    },
    serialize, subject, EventEnvelope,
};

#[test]
fn contract_event_envelope_preserves_public_fields_and_serializes_type() {
    let envelope = EventEnvelope::new(EventEnvelopeFields {
        id: "event-1".to_string(),
        r#type: "runtime.agent.ready".to_string(),
        subject: "subject.created".to_string(),
        source: "protocol-contract".to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        schema: "subject.v1".to_string(),
        scope: Some(json!({
            "runtime": "protocol",
            "tenant": "test",
            "segments": ["mvp", "contract"]
        })),
        refs: Some(json!([
            { "kind": "task", "id": "task-1" },
            { "kind": "agent", "id": "agent-1", "capabilities": ["shell", "llm"] }
        ])),
        payload: json!({ "name": "example", "nested": { "count": 2 } }),
        correlation_id: Some("correlation-1".to_string()),
        causation_id: Some("causation-1".to_string()),
        metadata: json!({ "tenant": "test", "trace": { "sampled": true } }),
    });

    assert_eq!(envelope.id, "event-1");
    assert_eq!(envelope.r#type, "runtime.agent.ready");
    assert_eq!(envelope.subject, "subject.created");
    assert_eq!(envelope.source, "protocol-contract");
    assert_eq!(envelope.timestamp, "2026-01-01T00:00:00Z");
    assert_eq!(envelope.schema, "subject.v1");
    assert_eq!(
        envelope.scope,
        Some(json!({
            "runtime": "protocol",
            "tenant": "test",
            "segments": ["mvp", "contract"]
        }))
    );
    assert_eq!(
        envelope.refs,
        Some(json!([
            { "kind": "task", "id": "task-1" },
            { "kind": "agent", "id": "agent-1", "capabilities": ["shell", "llm"] }
        ]))
    );
    assert_eq!(
        envelope.payload,
        json!({ "name": "example", "nested": { "count": 2 } })
    );
    assert_eq!(envelope.correlation_id, Some("correlation-1".to_string()));
    assert_eq!(envelope.causation_id, Some("causation-1".to_string()));
    assert_eq!(
        envelope.metadata,
        json!({ "tenant": "test", "trace": { "sampled": true } })
    );

    let serialized = serialize(&envelope).expect("envelope serializes to JSON");
    let serialized_value: Value =
        serde_json::from_str(&serialized).expect("serialized envelope is JSON");
    let deserialized = deserialize(&serialized).expect("serialized JSON deserializes to envelope");

    assert_eq!(serialized_value["type"], json!("runtime.agent.ready"));
    assert!(serialized_value.get("r#type").is_none());
    assert_eq!(
        serialized_value["scope"],
        json!({
            "runtime": "protocol",
            "tenant": "test",
            "segments": ["mvp", "contract"]
        })
    );
    assert_eq!(
        serialized_value["refs"],
        json!([
            { "kind": "task", "id": "task-1" },
            { "kind": "agent", "id": "agent-1", "capabilities": ["shell", "llm"] }
        ])
    );
    assert_eq!(
        serialized_value["payload"],
        json!({ "name": "example", "nested": { "count": 2 } })
    );
    assert_eq!(serialized_value["correlation_id"], json!("correlation-1"));
    assert_eq!(serialized_value["causation_id"], json!("causation-1"));
    assert_eq!(
        serialized_value["metadata"],
        json!({ "tenant": "test", "trace": { "sampled": true } })
    );
    assert_eq!(
        deserialized.scope,
        Some(json!({
            "runtime": "protocol",
            "tenant": "test",
            "segments": ["mvp", "contract"]
        }))
    );
    assert_eq!(
        deserialized.refs,
        Some(json!([
            { "kind": "task", "id": "task-1" },
            { "kind": "agent", "id": "agent-1", "capabilities": ["shell", "llm"] }
        ]))
    );
    assert_eq!(
        deserialized.payload,
        json!({ "name": "example", "nested": { "count": 2 } })
    );
    assert_eq!(
        deserialized.correlation_id,
        Some("correlation-1".to_string())
    );
    assert_eq!(deserialized.causation_id, Some("causation-1".to_string()));
    assert_eq!(
        deserialized.metadata,
        json!({ "tenant": "test", "trace": { "sampled": true } })
    );
    assert_eq!(deserialized, envelope);
}

#[test]
fn contract_event_envelope_deserializes_canonical_json_with_default_extensions() {
    let canonical_json = json!({
        "id": "event-canonical-1",
        "type": "runtime.task.assign",
        "subject": "runtime.task.assign.agent-1",
        "source": "protocol-contract",
        "timestamp": "2026-01-01T00:00:00Z",
        "schema": "runtime.event.v1",
        "payload": { "task_id": "task-1", "agent_id": "agent-1" },
        "correlation_id": null,
        "causation_id": null,
        "metadata": {}
    });

    let deserialized: EventEnvelope = serde_json::from_value(canonical_json)
        .expect("contract canonical envelope without current extensions deserializes");

    assert_eq!(deserialized.r#type, "runtime.task.assign");
    assert_eq!(deserialized.subject, "runtime.task.assign.agent-1");
    assert_eq!(deserialized.scope, None);
    assert_eq!(deserialized.refs, None);
}

#[test]
fn contract_runtime_subject_constants_match_canonical_values() {
    assert_eq!(subject::RUNTIME_AGENT_SPAWN, "runtime.agent.spawn");
    assert_eq!(subject::RUNTIME_AGENT_READY, "runtime.agent.ready");
    assert_eq!(subject::RUNTIME_AGENT_EXIT, "runtime.agent.exit");
    assert_eq!(subject::RUNTIME_AGENT_ERROR, "runtime.agent.error");
    assert_eq!(subject::RUNTIME_TASK_ASSIGN, "runtime.task.assign");
    assert_eq!(
        subject::RUNTIME_TASK_ASSIGN_AGENT_TEMPLATE,
        "runtime.task.assign.<agent_id>"
    );
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
    let agent_spawn = RuntimeAgentSpawnPayload {
        state: "BOOTING".to_string(),
    };
    let agent_spawn_json =
        serde_json::to_value(&agent_spawn).expect("agent spawn payload serializes");
    let agent_spawn_round_trip: RuntimeAgentSpawnPayload =
        serde_json::from_value(agent_spawn_json).expect("agent spawn payload deserializes");
    assert_eq!(agent_spawn_round_trip, agent_spawn);

    let task_assign = RuntimeTaskAssignPayload {
        task_id: "task-1".to_string(),
        input: json!({ "prompt": "run" }),
        capabilities: vec!["shell".to_string(), "llm".to_string()],
        metadata: json!({ "priority": "high" }),
        agent_id: Some("agent-1".to_string()),
        parent_task_id: Some("parent-task-1".to_string()),
        correlation_id: Some("correlation-1".to_string()),
    };
    let task_assign_json =
        serde_json::to_value(&task_assign).expect("task assign payload serializes");
    let task_assign_round_trip: RuntimeTaskAssignPayload =
        serde_json::from_value(task_assign_json).expect("task assign payload deserializes");
    assert_eq!(task_assign_round_trip, task_assign);
    assert_eq!(
        task_assign_round_trip.capabilities,
        vec!["shell".to_string(), "llm".to_string()]
    );

    let task_result = RuntimeTaskResultPayload {
        task_id: "task-1".to_string(),
        status: "completed".to_string(),
        result: Some(json!({ "ok": true })),
        error: json!({}),
        correlation_id: Some("correlation-1".to_string()),
    };
    let task_result_json =
        serde_json::to_value(&task_result).expect("task result payload serializes");
    let task_result_round_trip: RuntimeTaskResultPayload =
        serde_json::from_value(task_result_json).expect("task result payload deserializes");
    assert_eq!(task_result_round_trip, task_result);

    let agent_ready = RuntimeAgentReadyPayload {
        agent_id: "agent-1".to_string(),
        capabilities: vec!["shell".to_string()],
    };
    let agent_ready_json =
        serde_json::to_value(&agent_ready).expect("agent ready payload serializes");
    let agent_ready_round_trip: RuntimeAgentReadyPayload =
        serde_json::from_value(agent_ready_json).expect("agent ready payload deserializes");
    assert_eq!(agent_ready_round_trip, agent_ready);

    let agent_exit = RuntimeAgentExitPayload {
        agent_id: "agent-1".to_string(),
        reason: Some("shutdown".to_string()),
    };
    let agent_exit_json = serde_json::to_value(&agent_exit).expect("agent exit payload serializes");
    let agent_exit_round_trip: RuntimeAgentExitPayload =
        serde_json::from_value(agent_exit_json).expect("agent exit payload deserializes");
    assert_eq!(agent_exit_round_trip, agent_exit);

    let tool_request = RuntimeToolRequestPayload {
        task_id: "task-1".to_string(),
        capability: "shell".to_string(),
        input: json!({ "cmd": "echo ok" }),
        timeout_seconds: Some(30),
        correlation_id: Some("correlation-1".to_string()),
    };
    let tool_request_json =
        serde_json::to_value(&tool_request).expect("tool request payload serializes");
    let tool_request_round_trip: RuntimeToolRequestPayload =
        serde_json::from_value(tool_request_json).expect("tool request payload deserializes");
    assert_eq!(tool_request_round_trip, tool_request);

    let tool_result = RuntimeToolResultPayload {
        task_id: "task-1".to_string(),
        capability: "shell".to_string(),
        result: Some(json!({ "stdout": "ok" })),
        error: json!({}),
        correlation_id: Some("correlation-1".to_string()),
    };
    let tool_result_json =
        serde_json::to_value(&tool_result).expect("tool result payload serializes");
    let tool_result_round_trip: RuntimeToolResultPayload =
        serde_json::from_value(tool_result_json).expect("tool result payload deserializes");
    assert_eq!(tool_result_round_trip, tool_result);

    let llm_request = RuntimeLLMRequestPayload {
        task_id: "task-2".to_string(),
        model: Some("model-a".to_string()),
        prompt: Some("summarize".to_string()),
        messages: vec![json!({ "role": "user", "content": "summarize" })],
        options: json!({ "temperature": 0.2, "max_tokens": 128 }),
        correlation_id: Some("correlation-2".to_string()),
    };
    let llm_request_json =
        serde_json::to_value(&llm_request).expect("llm request payload serializes");
    let llm_request_round_trip: RuntimeLLMRequestPayload =
        serde_json::from_value(llm_request_json).expect("llm request payload deserializes");
    assert_eq!(llm_request_round_trip, llm_request);

    let llm_response = RuntimeLLMResponsePayload {
        task_id: "task-2".to_string(),
        model: Some("model-a".to_string()),
        message: json!({ "role": "assistant", "content": "summary" }),
        usage: json!({ "input_tokens": 10, "output_tokens": 2 }),
        error: json!({}),
        correlation_id: Some("correlation-2".to_string()),
    };
    let llm_response_json =
        serde_json::to_value(&llm_response).expect("llm response payload serializes");
    let llm_response_round_trip: RuntimeLLMResponsePayload =
        serde_json::from_value(llm_response_json).expect("llm response payload deserializes");
    assert_eq!(llm_response_round_trip, llm_response);

    let agent_error = RuntimeAgentErrorPayload {
        error: json!({ "message": "failed" }),
    };
    let agent_error_json =
        serde_json::to_value(&agent_error).expect("agent error payload serializes");
    let agent_error_round_trip: RuntimeAgentErrorPayload =
        serde_json::from_value(agent_error_json).expect("agent error payload deserializes");
    assert_eq!(agent_error_round_trip, agent_error);
}
