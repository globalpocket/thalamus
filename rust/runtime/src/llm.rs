use thalamus_protocol::payload::{RuntimeLLMRequestPayload, RuntimeLLMResponsePayload};

use crate::RuntimeError;

/// MockLlmProvider: 入力プロンプトから決定的なモック応答を返すLLMプロバイダ
///
/// response の request_id は request の request_id をそのまま保持する。
/// response の correlation_id は request の correlation_id をそのまま保持する。
#[derive(Debug, Default, Clone)]
pub struct MockLlmProvider;

impl MockLlmProvider {
    pub async fn complete(
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
