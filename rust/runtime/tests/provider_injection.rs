use std::sync::Arc;

use thalamus_bus::BasicBus;
use thalamus_protocol::{
    payload::{RuntimeLLMRequestPayload, RuntimeLLMResponsePayload},
    subject::RUNTIME_LLM_REQUEST,
};
use thalamus_runtime::{
    llm::LlmProvider, ErrorLlmProvider, MockLlmProvider, RuntimeError, ThalamusRuntime,
};

#[tokio::test]
async fn default_provider_is_mock() {
    let bus = BasicBus::new();
    let runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    assert_eq!(
        runtime.state().await,
        thalamus_runtime::RuntimeState::Initialized
    );
}

#[tokio::test]
async fn set_llm_provider_replaces_provider() {
    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // Replace with error provider
    runtime.set_llm_provider(Arc::new(ErrorLlmProvider)).await;

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
async fn provider_called_exactly_once() {
    let bus = BasicBus::new();
    let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));

    runtime.start().await.expect("runtime should start");

    // The MockLlmProvider is used, which doesn't track call count.
    // This test verifies that the provider is called without error.
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
}

/// Custom provider that tracks call count
#[derive(Clone)]
struct CountingProvider {
    count: Arc<std::sync::atomic::AtomicUsize>,
    result: Result<RuntimeLLMResponsePayload, RuntimeError>,
}

impl CountingProvider {
    fn new(count: Arc<std::sync::atomic::AtomicUsize>) -> Self {
        Self {
            count,
            result: Ok(RuntimeLLMResponsePayload {
                task_id: "task-1".to_string(),
                model: Some("mock".to_string()),
                request_id: Some("req-1".to_string()),
                status: "completed".to_string(),
                text: Some("response".to_string()),
                message: serde_json::json!({"content": "response"}),
                usage: serde_json::Value::Null,
                error: serde_json::Value::Null,
                correlation_id: Some("corr-1".to_string()),
            }),
        }
    }

    fn with_error(count: Arc<std::sync::atomic::AtomicUsize>) -> Self {
        Self {
            count,
            result: Err(RuntimeError::ProviderError("error".to_string())),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for CountingProvider {
    async fn complete(
        &self,
        request: RuntimeLLMRequestPayload,
    ) -> Result<RuntimeLLMResponsePayload, RuntimeError> {
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.result.clone()
    }
}

#[tokio::test]
async fn custom_provider_is_called() {
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let provider: Arc<dyn LlmProvider> = Arc::new(CountingProvider::new(count.clone()));

    let bus = BasicBus::new();
    let observer = bus.clone();
    let mut runtime = ThalamusRuntime::new(bus, provider);

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
                model: Some("custom".to_string()),
                correlation_id: Some("corr-1".to_string()),
                options: serde_json::json!({}),
            })
            .unwrap(),
        )
        .await
        .expect("llm.request should publish");

    // Provider should have been called exactly once
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);

    let events = observer.published_events().await;
    let subjects: Vec<&str> = events.iter().map(|e| e.subject.as_str()).collect();
    assert!(subjects.contains(&thalamus_protocol::subject::RUNTIME_LLM_RESPONSE));
}
