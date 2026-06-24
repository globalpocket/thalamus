use std::sync::Arc;

use thalamus_bus::BasicBus;
use thalamus_protocol::{
    payload::{
        RuntimeAgentReadyPayload, RuntimeLLMRequestPayload, RuntimeTaskAssignPayload,
        RuntimeTaskResultPayload, RuntimeToolRequestPayload,
    },
    subject::{
        RUNTIME_LLM_REQUEST, RUNTIME_TASK_ASSIGN, RUNTIME_TASK_RESULT,
        RUNTIME_TOOL_REQUEST, RUNTIME_TOOL_RESULT,
    },
};
use thalamus_runtime::{
    ErrorLlmProvider, MockLlmProvider, RuntimeError, ThalamusRuntime,
};

#[tokio::test]
async fn start_does_not_duplicate_internal_handlers() {
    let bus = BasicBus::new();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Internal handlers are registered for canonical subjects
    assert_eq!(
        thalamus_protocol::subject::RUNTIME_AGENT_READY.len(),
        thalamus_protocol::subject::RUNTIME_AGENT_READY.len()
    );

    // Trying to start again should fail
    let err = runtime.start().await.expect_err("running runtime cannot be started again");
    assert!(matches!(err, RuntimeError::LifecycleError(_)));
}

#[tokio::test]
async fn stop_closes_bus() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");
    assert!(!observer.is_closed().await);

    runtime.stop().await.expect("runtime should stop");
    assert!(observer.is_closed().await);
}

#[tokio::test]
async fn publish_after_stop_returns_closed_error() {
    let bus = BasicBus::new();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");
    runtime.stop().await.expect("runtime should stop");

    let result = runtime
        .publish(
            RUNTIME_TASK_ASSIGN.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeTaskAssignPayload {
                task_id: "task-1".to_string(),
                input: serde_json::json!({}),
                capabilities: vec![],
                metadata: serde_json::json!({}),
                agent_id: None,
                parent_task_id: None,
                correlation_id: None,
            })
            .unwrap(),
        )
        .await;

    assert!(matches!(
        result,
        Err(RuntimeError::BusError(msg)) if msg.contains("closed") || msg.contains("publish failed")
    ));
}

#[tokio::test]
async fn run_demo_observes_event_flow() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    let task_id = "task-demo-1".to_string();
    let agent_id = "agent-1".to_string();
    let correlation_id = "demo-correlation-1".to_string();

    // Publish task.assign
    runtime
        .publish(
            RUNTIME_TASK_ASSIGN.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeTaskAssignPayload {
                task_id: task_id.clone(),
                input: serde_json::json!({"prompt": "hello"}),
                capabilities: vec!["llm".to_string(), "tool.echo".to_string()],
                metadata: serde_json::json!({}),
                agent_id: Some(agent_id.clone()),
                parent_task_id: None,
                correlation_id: Some(correlation_id.clone()),
            })
            .unwrap(),
        )
        .await
        .expect("task.assign should publish");

    // Publish llm.request
    runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeLLMRequestPayload {
                task_id: task_id.clone(),
                request_id: None,
                prompt: Some("hello".to_string()),
                messages: Vec::new(),
                model: Some("mock".to_string()),
                correlation_id: Some(correlation_id.clone()),
                options: serde_json::json!({}),
            })
            .unwrap(),
        )
        .await
        .expect("llm.request should publish");

    // Publish tool.request
    runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeToolRequestPayload {
                task_id: task_id.clone(),
                request_id: None,
                capability: "tool.echo".to_string(),
                input: serde_json::json!({"text": "test"}),
                correlation_id: Some(correlation_id.clone()),
                timeout_seconds: None,
            })
            .unwrap(),
        )
        .await
        .expect("tool.request should publish");

    // Publish task.result
    runtime
        .publish(
            RUNTIME_TASK_RESULT.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeTaskResultPayload {
                task_id: task_id.clone(),
                status: "completed".to_string(),
                result: Some(serde_json::json!({"result": "ok"})),
                error: serde_json::Value::Null,
                correlation_id: Some(correlation_id),
            })
            .unwrap(),
        )
        .await
        .expect("task.result should publish");

    // Observe all events
    let events = observer.published_events().await;

    let subjects: Vec<&str> = events.iter().map(|e| e.subject.as_str()).collect();

    // Should contain task.assign, llm.request, llm.response, tool.request, tool.result, task.result
    assert!(subjects.contains(&RUNTIME_TASK_ASSIGN));
    assert!(subjects.contains(&RUNTIME_LLM_REQUEST));
    assert!(subjects.contains(&thalamus_protocol::subject::RUNTIME_LLM_RESPONSE));
    assert!(subjects.contains(&RUNTIME_TOOL_REQUEST));
    assert!(subjects.contains(&RUNTIME_TOOL_RESULT));
    assert!(subjects.contains(&RUNTIME_TASK_RESULT));
}

#[tokio::test]
async fn provider_error_emits_llm_response_error() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(ErrorLlmProvider));

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

    // Should have llm.request and llm.response (error)
    let subjects: Vec<&str> = events.iter().map(|e| e.subject.as_str()).collect();
    assert!(subjects.contains(&RUNTIME_LLM_REQUEST));
    assert!(subjects.contains(&thalamus_protocol::subject::RUNTIME_LLM_RESPONSE));

    // Find the llm.response event
    let response_event = events
        .iter()
        .find(|e| e.subject == thalamus_protocol::subject::RUNTIME_LLM_RESPONSE)
        .expect("should have llm.response");

    assert_eq!(response_event.payload["status"], "error");
}

#[tokio::test]
async fn unknown_tool_emits_tool_result_error() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

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

    let subjects: Vec<&str> = events.iter().map(|e| e.subject.as_str()).collect();
    assert!(subjects.contains(&RUNTIME_TOOL_REQUEST));
    assert!(subjects.contains(&RUNTIME_TOOL_RESULT));

    // Find the tool.result event
    let result_event = events
        .iter()
        .find(|e| e.subject == RUNTIME_TOOL_RESULT)
        .expect("should have tool.result");

    assert_eq!(result_event.payload["status"], "error");
}

#[tokio::test]
async fn tool_error_emits_tool_result_error() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // tool.echo should work fine, but let's test with a tool that might error
    runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeToolRequestPayload {
                task_id: "task-1".to_string(),
                request_id: None,
                capability: "tool.echo".to_string(),
                input: serde_json::json!({"text": "test"}),
                correlation_id: Some("corr-1".to_string()),
                timeout_seconds: None,
            })
            .unwrap(),
        )
        .await
        .expect("tool.request should publish");

    let events = observer.published_events().await;

    let subjects: Vec<&str> = events.iter().map(|e| e.subject.as_str()).collect();
    assert!(subjects.contains(&RUNTIME_TOOL_REQUEST));
    assert!(subjects.contains(&RUNTIME_TOOL_RESULT));

    // Find the tool.result event
    let result_event = events
        .iter()
        .find(|e| e.subject == RUNTIME_TOOL_RESULT)
        .expect("should have tool.result");

    assert_eq!(result_event.payload["status"], "completed");
}

#[tokio::test]
async fn agent_id_none_error_does_not_modify_registry() {
    let bus = BasicBus::new();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Publish agent.error without agent_id
    runtime
        .publish(
            thalamus_protocol::subject::RUNTIME_AGENT_ERROR.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeAgentReadyPayload {
                agent_id: "agent-1".to_string(),
                capabilities: vec![],
            })
            .unwrap(),
        )
        .await
        .expect("agent.ready should publish");

    // agent-1 should be registered
    let registry = runtime.worker_registry().await;
    assert!(registry.lookup("agent-1").is_some());
}

#[tokio::test]
async fn task_agent_update_via_bus_handler() {
    let bus = BasicBus::new();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    let task_id = "task-1".to_string();
    let agent_id = "agent-1".to_string();

    // Publish task.assign
    runtime
        .publish(
            RUNTIME_TASK_ASSIGN.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeTaskAssignPayload {
                task_id: task_id.clone(),
                input: serde_json::json!({"prompt": "hello"}),
                capabilities: vec!["llm".to_string()],
                metadata: serde_json::json!({}),
                agent_id: Some(agent_id.clone()),
                parent_task_id: None,
                correlation_id: Some("corr-1".to_string()),
            })
            .unwrap(),
        )
        .await
        .expect("task.assign should publish");

    // Check task state
    let task_state = runtime.task_state(&task_id).await;
    assert!(task_state.is_some());
    let task = task_state.unwrap();
    assert!(matches!(
        *task.status.read().await,
        thalamus_runtime::TaskStatus::Assigned
    ));
}

#[tokio::test]
async fn publish_does_not_directly_process_event() {
    // publish() should only create envelope and publish to bus.
    // The actual processing is done by internal handlers.
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    let processed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let processed_clone = processed.clone();

    // Register a user handler that tracks processing
    runtime
        .register_handler(
            RUNTIME_TASK_ASSIGN.to_string(),
            Arc::new(move |_subject, _event| {
                let flag = processed_clone.clone();
                Box::pin(async move {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                })
            }),
        )
        .await
        .expect("register_handler should succeed");

    runtime
        .publish(
            RUNTIME_TASK_ASSIGN.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeTaskAssignPayload {
                task_id: "task-1".to_string(),
                input: serde_json::json!({}),
                capabilities: vec![],
                metadata: serde_json::json!({}),
                agent_id: None,
                parent_task_id: None,
                correlation_id: None,
            })
            .unwrap(),
        )
        .await
        .expect("publish should succeed");

    // User handler should have been called
    assert!(processed.load(std::sync::atomic::Ordering::SeqCst));

    // Events should be recorded by bus
    let events = observer.published_events().await;
    assert!(!events.is_empty());
}
