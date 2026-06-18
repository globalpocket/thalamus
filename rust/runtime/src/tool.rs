use thalamus_protocol::payload::{RuntimeToolRequestPayload, RuntimeToolResultPayload};

use crate::RuntimeError;

/// EchoTool: 入力payloadをそのまま結果として返すツール
///
/// result の request_id は request の request_id をそのまま保持する。
/// result の correlation_id は request の correlation_id をそのまま保持する。
#[derive(Debug, Default, Clone)]
pub struct EchoTool;

impl EchoTool {
    pub async fn invoke(
        &self,
        request: RuntimeToolRequestPayload,
    ) -> Result<RuntimeToolResultPayload, RuntimeError> {
        Ok(RuntimeToolResultPayload {
            task_id: request.task_id.clone(),
            capability: request.capability,
            request_id: request.request_id.clone(),
            status: "completed".to_string(),
            output: Some(request.input.clone()),
            result: Some(request.input),
            error: serde_json::Value::Null,
            correlation_id: request.correlation_id,
        })
    }
}
