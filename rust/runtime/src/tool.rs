use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use thalamus_protocol::payload::{RuntimeToolRequestPayload, RuntimeToolResultPayload};

use crate::error::RuntimeError;

/// Tool: object-safe trait for tools.
///
/// `dyn Tool` does not require `Debug` to avoid forcing tool implementations
/// to expose their internal state.  ToolRegistry's manual Debug prints only
/// capability names.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Canonical capability name (e.g. "tool.echo").
    fn capability(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str {
        ""
    }

    /// Execute the tool with the given request.
    async fn invoke(
        &self,
        request: RuntimeToolRequestPayload,
    ) -> Result<RuntimeToolResultPayload, RuntimeError>;
}

/// EchoTool: echoes the input back.
///
/// Canonical capability name is `"tool.echo"`.  For backwards compatibility
/// it is also registered under the `"echo"` alias when added via
/// `ToolRegistry::register_alias`.
#[derive(Default)]
pub struct EchoTool;

impl fmt::Debug for EchoTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EchoTool").finish()
    }
}

#[async_trait]
impl Tool for EchoTool {
    fn capability(&self) -> &str {
        "tool.echo"
    }

    fn description(&self) -> &str {
        "Echoes the input back"
    }

    async fn invoke(
        &self,
        request: RuntimeToolRequestPayload,
    ) -> Result<RuntimeToolResultPayload, RuntimeError> {
        Ok(RuntimeToolResultPayload {
            task_id: request.task_id.clone(),
            capability: request.capability.clone(),
            request_id: request.request_id.clone(),
            status: "completed".to_string(),
            output: Some(request.input.clone()),
            result: Some(request.input.clone()),
            error: serde_json::json!({}),
            correlation_id: request.correlation_id.clone(),
        })
    }
}

/// ToolRegistry: tool registration and lookup using `Arc<dyn Tool>`.
///
/// Uses `Arc<dyn Tool>` instead of `Box<dyn Tool>` to avoid cloning.
/// `register_alias()` clones the Arc reference.
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool by its canonical capability name.
    pub fn register(&mut self, capability: String, tool: Arc<dyn Tool>) {
        self.tools.insert(capability, tool);
    }

    /// Register an alias that points to an existing tool.
    /// Returns true if the target was found.
    pub fn register_alias(&mut self, alias: String, target: String) -> bool {
        if let Some(tool) = self.tools.get(&target) {
            self.tools.insert(alias, Arc::clone(tool));
            true
        } else {
            false
        }
    }

    /// Remove a tool by capability name.
    pub fn unregister(&mut self, capability: &str) -> Option<Arc<dyn Tool>> {
        self.tools.remove(capability)
    }

    /// Lookup a tool by capability name.
    pub fn get(&self, capability: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(capability).cloned()
    }

    /// Return a sorted list of all registered capability names.
    pub fn list_capabilities(&self) -> Vec<String> {
        let mut caps: Vec<String> = self.tools.keys().cloned().collect();
        caps.sort();
        caps
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("capabilities", &self.list_capabilities())
            .finish_non_exhaustive()
    }
}
