use async_trait::async_trait;
use std::collections::HashMap;
use thalamus_protocol::payload::{RuntimeToolRequestPayload, RuntimeToolResultPayload};

use crate::RuntimeError;

/// Tool: pluggable tool trait
///
/// Runtime holds `ToolRegistry` and delegates tool invocation to it.
/// `EchoTool` is the default implementation that echoes back the input.
#[async_trait]
pub trait Tool: Send + Sync {
    async fn invoke(
        &self,
        request: RuntimeToolRequestPayload,
    ) -> Result<RuntimeToolResultPayload, RuntimeError>;
}

/// EchoTool: input payload returns as result unchanged
///
/// result.request_id preserves request.request_id.
/// result.correlation_id preserves request.correlation_id.
#[derive(Debug, Default, Clone)]
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    async fn invoke(
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

/// ToolRegistry: registry of tools by capability name
///
/// Allows runtime to look up tools by capability string.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
}

impl ToolRegistry {
    /// Create a new empty ToolRegistry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool by capability name
    pub fn register(&mut self, capability: String, tool: Box<dyn Tool>) {
        self.tools.insert(capability, tool);
    }

    /// Look up a tool by capability name
    pub fn get(&self, capability: &str) -> Option<&dyn Tool> {
        self.tools.get(capability).map(|t| t.as_ref())
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}
