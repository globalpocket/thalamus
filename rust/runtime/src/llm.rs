use async_trait::async_trait;
use thalamus_protocol::payload::{RuntimeLLMRequestPayload, RuntimeLLMResponsePayload};

use crate::RuntimeError;

/// LlmProvider: pluggable LLM provider trait
///
/// Runtime holds `Arc<dyn LlmProvider>` and delegates completion to it.
/// Default is `MockLlmProvider`. Custom providers can be injected via
/// `ThalamusRuntime::set_llm_provider()`.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(
        &self,
        request: RuntimeLLMRequestPayload,
    ) -> Result<RuntimeLLMResponsePayload, RuntimeError>;
}

/// MockLlmProvider: returns a deterministic mock response
///
/// response.request_id preserves request.request_id.
/// response.correlation_id preserves request.correlation_id.
#[derive(Debug, Default, Clone)]
pub struct MockLlmProvider;

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(
        &self,
        request: RuntimeLLMRequestPayload,
    ) -> Result<RuntimeLLMResponsePayload, RuntimeError> {
        let response_input = request.prompt.unwrap_or_else(|| {
            request
                .messages
                .last()
                .and_then(|message| message.get("content"))
                .and_then(|content| content.as_str())
                .unwrap_or_default()
                .to_string()
        });

        let text = format!("Mock response: {}", response_input);
        Ok(RuntimeLLMResponsePayload {
            task_id: request.task_id.clone(),
            model: request.model.or_else(|| Some("mock".to_string())),
            request_id: request.request_id.clone(),
            status: "completed".to_string(),
            text: Some(text.clone()),
            message: serde_json::json!({
                "content": text
            }),
            usage: serde_json::Value::Null,
            error: serde_json::Value::Null,
            correlation_id: request.correlation_id,
        })
    }
}

/// ErrorProvider: always returns an error for testing error paths
#[derive(Debug, Default, Clone)]
pub struct ErrorLlmProvider;

#[async_trait]
impl LlmProvider for ErrorLlmProvider {
    async fn complete(
        &self,
        request: RuntimeLLMRequestPayload,
    ) -> Result<RuntimeLLMResponsePayload, RuntimeError> {
        Ok(RuntimeLLMResponsePayload {
            task_id: request.task_id.clone(),
            model: request.model.or_else(|| Some("error-mock".to_string())),
            request_id: request.request_id.clone(),
            status: "error".to_string(),
            text: None,
            message: serde_json::json!({}),
            usage: serde_json::Value::Null,
            error: serde_json::json!({
                "kind": "provider_error",
                "message": "simulated provider failure"
            }),
            correlation_id: request.correlation_id,
        })
    }
}
