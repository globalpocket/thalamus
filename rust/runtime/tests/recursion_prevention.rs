use std::sync::Arc;

use thalamus_bus::BasicBus;
use thalamus_protocol::{
    payload::{
        RuntimeAgentSpawnPayload, RuntimeLLMRequestPayload, RuntimeTaskAssignPayload,
        RuntimeToolRequestPayload,
    },
    subject::{
        RUNTIME_AGENT_SPAWN, RUNTIME_LLM_REQUEST, RUNTIME_LLM_RESPONSE, RUNTIME_TASK_ASSIGN,
        RUNTIME_TOOL_REQUEST, RUNTIME_TOOL_RESULT,
    },
};
use thalamus_runtime::MockLlmProvider;

/// runtime.agent.spawn ハンドラはイベントを再publishしない
#[tokio::test]
async fn spawn_handler_does_not_republish() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Publish runtime.agent.spawn event with valid payload
    runtime
        .publish(
            RUNTIME_AGENT_SPAWN.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeAgentSpawnPayload {
                state: "running".to_string(),
            })
            .unwrap(),
        )
        .await
        .expect("spawn should publish");

    // Only the original event should be recorded, no re-published event
    let events = observer.published_events().await;
    let spawn_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_AGENT_SPAWN)
        .count();
    assert_eq!(spawn_count, 1, "spawn event should not be re-published");
}

/// LLMリクエストのパースエラー時に再publishしない
#[tokio::test]
async fn llm_request_parse_error_does_not_republish() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Publish invalid LLM request (missing required field: task_id)
    // This should fail validation at publish() level, not reach the handler
    let result = runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "test".to_string(),
            serde_json::json!({"invalid": "payload"}),
        )
        .await;

    // Validation should reject this at publish level
    assert!(
        result.is_err(),
        "invalid payload should be rejected at publish"
    );

    // No events should be recorded because publish() returned error
    let events = observer.published_events().await;
    let llm_request_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_LLM_REQUEST)
        .count();
    assert_eq!(
        llm_request_count, 0,
        "invalid llm.request should not be recorded"
    );
}

/// Toolリクエストのパースエラー時に再publishしない
#[tokio::test]
async fn tool_request_parse_error_does_not_republish() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Publish invalid tool request (missing required field: task_id)
    // This should fail validation at publish() level
    let result = runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "test".to_string(),
            serde_json::json!({"invalid": "payload"}),
        )
        .await;

    // Validation should reject this at publish level
    assert!(
        result.is_err(),
        "invalid payload should be rejected at publish"
    );

    // No events should be recorded because publish() returned error
    let events = observer.published_events().await;
    let tool_request_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_TOOL_REQUEST)
        .count();
    assert_eq!(
        tool_request_count, 0,
        "invalid tool.request should not be recorded"
    );
}

/// unknown tool リクエストはエラーレスポンスをpublishするが、元のリクエストは再publishしない
#[tokio::test]
async fn unknown_tool_publishes_error_not_request() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Publish tool request for unknown capability with valid payload
    runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeToolRequestPayload {
                task_id: "task-1".to_string(),
                request_id: None,
                capability: "unknown.tool".to_string(),
                input: serde_json::json!({}),
                correlation_id: Some("corr-1".to_string()),
                timeout_seconds: None,
            })
            .unwrap(),
        )
        .await
        .expect("tool.request should publish");

    let events = observer.published_events().await;
    let tool_request_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_TOOL_REQUEST)
        .count();
    let tool_result_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_TOOL_RESULT)
        .count();

    // Original request should not be re-published (handler drops it after processing)
    assert_eq!(
        tool_request_count, 1,
        "tool.request should appear once (original publish only)"
    );
    // Error response should be published
    assert_eq!(
        tool_result_count, 1,
        "tool.result error should be published for unknown tool"
    );

    // Verify the error response contains the capability field
    let error_event = events
        .iter()
        .find(|e| e.subject == RUNTIME_TOOL_RESULT)
        .expect("should have tool.result event");
    assert!(
        error_event.payload["capability"].is_string(),
        "error response should contain capability field"
    );
    assert_eq!(
        error_event.payload["capability"], "unknown.tool",
        "capability should match the unknown tool name"
    );
}

/// LLMリクエスト→レスポンスのサイクルで再帰が発生しない
#[tokio::test]
async fn llm_request_response_cycle_no_recursion() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeLLMRequestPayload {
                task_id: "task-1".to_string(),
                request_id: None,
                prompt: Some("hello".to_string()),
                messages: Vec::new(),
                model: Some("mock".to_string()),
                correlation_id: Some("corr-1".to_string()),
                options: serde_json::json!({}),
            })
            .unwrap(),
        )
        .await
        .expect("llm.request should publish");

    let events = observer.published_events().await;

    // Should have exactly one request and one response
    let llm_request_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_LLM_REQUEST)
        .count();
    let llm_response_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_LLM_RESPONSE)
        .count();

    assert_eq!(llm_request_count, 1, "exactly one llm.request");
    assert_eq!(llm_response_count, 1, "exactly one llm.response");
}

/// task.assign → llm.request → llm.response のチェーンで再帰が発生しない
#[tokio::test]
async fn task_chain_no_recursion() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Start a task chain
    runtime
        .publish(
            RUNTIME_TASK_ASSIGN.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeTaskAssignPayload {
                task_id: "task-1".to_string(),
                input: serde_json::json!({"prompt": "hello"}),
                capabilities: vec!["llm".to_string()],
                metadata: serde_json::json!({}),
                agent_id: None,
                parent_task_id: None,
                correlation_id: Some("corr-1".to_string()),
            })
            .unwrap(),
        )
        .await
        .expect("task.assign should publish");

    // Then trigger LLM
    runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeLLMRequestPayload {
                task_id: "task-1".to_string(),
                request_id: None,
                prompt: Some("hello".to_string()),
                messages: Vec::new(),
                model: Some("mock".to_string()),
                correlation_id: Some("corr-1".to_string()),
                options: serde_json::json!({}),
            })
            .unwrap(),
        )
        .await
        .expect("llm.request should publish");

    let events = observer.published_events().await;

    // Count events by subject
    let task_assign_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_TASK_ASSIGN)
        .count();
    let llm_request_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_LLM_REQUEST)
        .count();
    let llm_response_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_LLM_RESPONSE)
        .count();

    // Each event should appear exactly once (no recursion)
    assert_eq!(task_assign_count, 1, "exactly one task.assign");
    assert_eq!(llm_request_count, 1, "exactly one llm.request");
    assert_eq!(llm_response_count, 1, "exactly one llm.response");
}
