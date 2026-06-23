use std::sync::Arc;

use thalamus_bus::BasicBus;
use thalamus_protocol::{
    payload::{RuntimeToolRequestPayload, RuntimeToolResultPayload},
    subject::RUNTIME_TOOL_REQUEST,
};
use thalamus_runtime::{Tool, ThalamusRuntime};

#[tokio::test]
async fn tool_echo_returns_input() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(thalamus_runtime::MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    let input = serde_json::json!({"text": "hello"});
    runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeToolRequestPayload {
                task_id: "task-1".to_string(),
                request_id: None,
                capability: "tool.echo".to_string(),
                input: input.clone(),
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
    assert!(subjects.contains(&thalamus_protocol::subject::RUNTIME_TOOL_RESULT));

    // Find the tool.result event
    let result_event = events
        .iter()
        .find(|e| e.subject == thalamus_protocol::subject::RUNTIME_TOOL_RESULT)
        .expect("should have tool.result");

    assert_eq!(result_event.payload["status"], "completed");
    assert_eq!(result_event.payload["result"], input);
}

#[tokio::test]
async fn tool_echo_alias_works() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(thalamus_runtime::MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Use "echo" alias instead of "tool.echo"
    runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "test".to_string(),
            serde_json::to_value(RuntimeToolRequestPayload {
                task_id: "task-1".to_string(),
                request_id: None,
                capability: "echo".to_string(),
                input: serde_json::json!({"key": "value"}),
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
    assert!(subjects.contains(&thalamus_protocol::subject::RUNTIME_TOOL_RESULT));

    let result_event = events
        .iter()
        .find(|e| e.subject == thalamus_protocol::subject::RUNTIME_TOOL_RESULT)
        .expect("should have tool.result");

    assert_eq!(result_event.payload["status"], "completed");
}

#[tokio::test]
async fn register_tool_adds_capability() {
    let bus = BasicBus::new();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(thalamus_runtime::MockLlmProvider));

    // Register a custom tool
    struct DoubleTool;

    #[async_trait::async_trait]
    impl Tool for DoubleTool {
        fn name(&self) -> &str {
            "double"
        }

        fn description(&self) -> &str {
            "Doubles the input"
        }

        async fn invoke(&self, parameters: serde_json::Value) -> Result<serde_json::Value, thalamus_runtime::RuntimeError> {
            if let Some(n) = parameters["value"].as_i64() {
                Ok(serde_json::json!({"result": n * 2}))
            } else {
                Ok(serde_json::json!({"error": "invalid input"}))
            }
        }
    }

    runtime.register_tool("math.double".to_string(), Box::new(DoubleTool)).await;

    let caps = runtime.list_tool_capabilities().await;
    assert!(caps.contains(&"echo".to_string()));
    assert!(caps.contains(&"math.double".to_string()));
    assert!(caps.contains(&"tool.echo".to_string()));
}

#[tokio::test]
async fn unregister_tool_removes_capability() {
    let bus = BasicBus::new();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(thalamus_runtime::MockLlmProvider));

    runtime.register_tool("math.double".to_string(), Box::new(thalamus_runtime::EchoTool)).await;

    let caps = runtime.list_tool_capabilities().await;
    assert!(caps.contains(&"math.double".to_string()));

    runtime.unregister_tool("math.double").await;

    let caps = runtime.list_tool_capabilities().await;
    assert!(!caps.contains(&"math.double".to_string()));
}

#[tokio::test]
async fn tool_called_exactly_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let call_count = Arc::new(AtomicUsize::new(0));
    let count_clone = call_count.clone();

    struct CountingTool {
        count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Tool for CountingTool {
        fn name(&self) -> &str {
            "counting"
        }

        fn description(&self) -> &str {
            "Counts invocations"
        }

        async fn invoke(&self, _parameters: serde_json::Value) -> Result<serde_json::Value, thalamus_runtime::RuntimeError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(serde_json::json!({"count": self.count.load(Ordering::SeqCst)}))
        }
    }

    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(thalamus_runtime::MockLlmProvider));

    runtime.register_tool("test.counting".to_string(), Box::new(CountingTool { count: count_clone })).await;

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

    // Tool should have been called exactly once
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    let events = observer.published_events().await;
    let subjects: Vec<&str> = events.iter().map(|e| e.subject.as_str()).collect();
    assert!(subjects.contains(&thalamus_protocol::subject::RUNTIME_TOOL_RESULT));
}

#[tokio::test]
async fn one_request_emits_one_response() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(thalamus_runtime::MockLlmProvider));

    runtime.start().await.expect("runtime should start");

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

    // Count tool.result events
    let result_count = events
        .iter()
        .filter(|e| e.subject == thalamus_protocol::subject::RUNTIME_TOOL_RESULT)
        .count();

    assert_eq!(result_count, 1);
}
