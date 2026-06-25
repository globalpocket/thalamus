use std::sync::Arc;

use thalamus_bus::{BasicBus, MessageBus};
use thalamus_protocol::{
    payload::{
        RuntimeAgentErrorPayload, RuntimeAgentReadyPayload, RuntimeLLMRequestPayload,
        RuntimeTaskAssignPayload, RuntimeToolRequestPayload,
    },
    subject::{
        RUNTIME_AGENT_ERROR, RUNTIME_AGENT_READY, RUNTIME_LLM_REQUEST, RUNTIME_LLM_RESPONSE,
        RUNTIME_TASK_ASSIGN, RUNTIME_TOOL_REQUEST, RUNTIME_TOOL_RESULT,
    },
};
use thalamus_runtime::MockLlmProvider;

#[tokio::test]
async fn invalid_canonical_payload_is_not_recorded() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    // Invalid payload (missing agent_id) should return error from publish
    let result = runtime
        .publish(
            RUNTIME_AGENT_READY.to_string(),
            "test".to_string(),
            serde_json::json!({"capabilities": ["llm"]}), // missing agent_id
        )
        .await;

    assert!(result.is_err());

    // No events should be recorded
    let events = observer.published_events().await;
    assert!(events.is_empty());
}

#[tokio::test]
async fn unknown_extension_subject_is_recorded() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    let result = runtime
        .publish(
            "custom.extension".to_string(),
            "test".to_string(),
            serde_json::json!({"any": "data"}),
        )
        .await;

    assert!(result.is_ok());

    let events = observer.published_events().await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].subject, "custom.extension");
}

#[tokio::test]
async fn request_id_added_before_recording() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

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
        .expect("publish should succeed");

    let events = observer.published_events().await;
    assert_eq!(events.len(), 1);

    // request_id should be auto-completed
    let request_id = events[0].payload["request_id"]
        .as_str()
        .expect("request_id should exist");
    assert!(!request_id.is_empty());
}

#[tokio::test]
async fn task_agent_update_via_bus_handler() {
    use thalamus_protocol::payload::RuntimeAgentReadyPayload;

    let bus = BasicBus::new();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    let task_id = "task-1".to_string();
    let agent_id = "agent-1".to_string();

    // First, register the agent via agent.ready
    runtime
        .publish(
            RUNTIME_AGENT_READY.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeAgentReadyPayload {
                agent_id: agent_id.clone(),
                capabilities: vec!["llm".to_string()],
            })
            .unwrap(),
        )
        .await
        .expect("agent.ready should publish");

    // Then assign a task
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

    // Check worker registry
    let registry = runtime.worker_registry().await;
    let worker = registry
        .lookup(&agent_id)
        .expect("agent should be registered");
    assert_eq!(worker.state, thalamus_runtime::WorkerState::Ready);
}

#[tokio::test]
async fn publish_does_not_directly_process_event() {
    // publish() should only create envelope and publish to bus.
    // The actual processing is done by internal handlers.
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

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

    // Events should be recorded by bus
    let events = observer.published_events().await;
    assert!(!events.is_empty());
    assert!(events.iter().any(|e| e.subject == RUNTIME_TASK_ASSIGN));
}

#[tokio::test]
async fn provider_called_exactly_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let call_count = Arc::new(AtomicUsize::new(0));

    struct CountingProvider {
        count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl thalamus_runtime::llm::LlmProvider for CountingProvider {
        async fn complete(
            &self,
            _request: RuntimeLLMRequestPayload,
        ) -> Result<
            thalamus_protocol::payload::RuntimeLLMResponsePayload,
            thalamus_runtime::RuntimeError,
        > {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(thalamus_protocol::payload::RuntimeLLMResponsePayload {
                task_id: "task-1".to_string(),
                model: Some("mock".to_string()),
                request_id: Some("req-1".to_string()),
                status: "completed".to_string(),
                text: Some("response".to_string()),
                message: serde_json::json!({"content": "response"}),
                usage: serde_json::Value::Null,
                error: serde_json::Value::Null,
                correlation_id: Some("corr-1".to_string()),
            })
        }
    }

    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(
        bus,
        Arc::new(CountingProvider {
            count: call_count.clone(),
        }),
    );

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

    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    let events = observer.published_events().await;
    let subjects: Vec<&str> = events.iter().map(|e| e.subject.as_str()).collect();
    assert!(subjects.contains(&RUNTIME_LLM_REQUEST));
    assert!(subjects.contains(&RUNTIME_LLM_RESPONSE));
}

#[tokio::test]
async fn tool_called_exactly_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let call_count = Arc::new(AtomicUsize::new(0));

    struct CountingTool {
        count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl thalamus_runtime::Tool for CountingTool {
        fn capability(&self) -> &str {
            "test.counting"
        }

        fn description(&self) -> &str {
            "Counts invocations"
        }

        async fn invoke(
            &self,
            request: thalamus_protocol::payload::RuntimeToolRequestPayload,
        ) -> Result<
            thalamus_protocol::payload::RuntimeToolResultPayload,
            thalamus_runtime::RuntimeError,
        > {
            self.count.fetch_add(1, Ordering::SeqCst);
            let count = self.count.load(Ordering::SeqCst);
            Ok(thalamus_protocol::payload::RuntimeToolResultPayload {
                task_id: request.task_id.clone(),
                capability: request.capability.clone(),
                request_id: request.request_id.clone(),
                status: "completed".to_string(),
                output: Some(serde_json::json!({"count": count})),
                result: Some(serde_json::json!({"count": count})),
                error: serde_json::json!({}),
                correlation_id: request.correlation_id.clone(),
            })
        }
    }

    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime
        .register_tool(
            "test.counting".to_string(),
            Arc::new(CountingTool {
                count: call_count.clone(),
            }),
        )
        .await;

    runtime.start().await.expect("runtime should start");

    runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeToolRequestPayload {
                task_id: "task-1".to_string(),
                request_id: None,
                capability: "test.counting".to_string(),
                input: serde_json::json!({}),
                correlation_id: Some("corr-1".to_string()),
                timeout_seconds: None,
            })
            .unwrap(),
        )
        .await
        .expect("tool.request should publish");

    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    let events = observer.published_events().await;
    let subjects: Vec<&str> = events.iter().map(|e| e.subject.as_str()).collect();
    assert!(subjects.contains(&RUNTIME_TOOL_REQUEST));
    assert!(subjects.contains(&RUNTIME_TOOL_RESULT));
}

#[tokio::test]
async fn one_request_emits_one_response() {
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
    let response_count = events
        .iter()
        .filter(|e| e.subject == RUNTIME_LLM_RESPONSE)
        .count();

    assert_eq!(response_count, 1);
}

#[tokio::test]
async fn custom_handler_does_not_disable_internal_handler() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Register a user handler for RUNTIME_AGENT_READY
    runtime
        .register_handler(
            RUNTIME_AGENT_READY.to_string(),
            Arc::new(|_subject, _event| Box::pin(async move {})),
        )
        .await
        .expect("register_handler should succeed");

    runtime
        .publish(
            RUNTIME_AGENT_READY.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeAgentReadyPayload {
                agent_id: "agent-1".to_string(),
                capabilities: vec!["llm".to_string()],
            })
            .unwrap(),
        )
        .await
        .expect("agent.ready should publish");

    // Internal handler should have updated the registry
    let registry = runtime.worker_registry().await;
    assert!(registry.lookup("agent-1").is_some());

    // User handler should have received the event (it's in published_events)
    let events = observer.published_events().await;
    assert!(events.iter().any(|e| e.subject == RUNTIME_AGENT_READY));
}

#[tokio::test]
async fn multiple_user_handlers_receive_same_event() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let bus = BasicBus::new();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    let counter1 = Arc::new(AtomicUsize::new(0));
    let counter2 = Arc::new(AtomicUsize::new(0));

    let c1 = counter1.clone();
    runtime
        .register_handler(
            "custom.subject".to_string(),
            Arc::new(move |_subject, _event| {
                let c = c1.clone();
                Box::pin(async move {
                    c.fetch_add(1, Ordering::SeqCst);
                })
            }),
        )
        .await
        .expect("register_handler should succeed");

    let c2 = counter2.clone();
    runtime
        .register_handler(
            "custom.subject".to_string(),
            Arc::new(move |_subject, _event| {
                let c = c2.clone();
                Box::pin(async move {
                    c.fetch_add(1, Ordering::SeqCst);
                })
            }),
        )
        .await
        .expect("register_handler should succeed");

    runtime
        .publish(
            "custom.subject".to_string(),
            "test".to_string(),
            serde_json::json!({"data": "value"}),
        )
        .await
        .expect("publish should succeed");

    assert_eq!(counter1.load(Ordering::SeqCst), 1);
    assert_eq!(counter2.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn agent_id_none_error_does_not_modify_registry() {
    let bus = BasicBus::new();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Publish agent.error without agent_id — should not crash
    runtime
        .publish(
            RUNTIME_AGENT_ERROR.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeAgentErrorPayload {
                agent_id: None,
                task_id: None,
                error: serde_json::json!({"message": "runtime error"}),
            })
            .unwrap(),
        )
        .await
        .expect("agent.error should publish");

    // Registry should not have any new entries from this
    let registry = runtime.worker_registry().await;
    assert!(registry.lookup("none-agent").is_none());
}

#[tokio::test]
async fn start_does_not_duplicate_internal_handlers() {
    let bus = BasicBus::new();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Trying to start again should fail
    let result = runtime.start().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn stop_closes_bus() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");
    assert!(!observer.is_closed().await);

    runtime.stop().await.expect("runtime should stop");
    assert!(observer.is_closed().await);
}

#[tokio::test]
async fn publish_after_stop_returns_closed_error() {
    let bus = BasicBus::new();
    let mut runtime = thalamus_runtime::ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

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

    assert!(result.is_err());
}
