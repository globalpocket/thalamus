use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentSpawnPayload {
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTaskAssignPayload {
    pub task_id: String,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub metadata: Value,
    pub agent_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTaskResultPayload {
    pub task_id: String,
    pub status: String,
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Value,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentReadyPayload {
    pub agent_id: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentExitPayload {
    pub agent_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeToolRequestPayload {
    pub task_id: String,
    pub capability: String,
    pub input: Value,
    pub timeout_seconds: Option<u64>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeToolResultPayload {
    pub task_id: String,
    pub capability: String,
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Value,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeLLMRequestPayload {
    pub task_id: String,
    pub model: Option<String>,
    pub prompt: Option<String>,
    #[serde(default)]
    pub messages: Vec<Value>,
    #[serde(default)]
    pub options: Value,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeLLMResponsePayload {
    pub task_id: String,
    pub model: Option<String>,
    #[serde(default)]
    pub message: Value,
    #[serde(default)]
    pub usage: Value,
    #[serde(default)]
    pub error: Value,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentErrorPayload {
    #[serde(default)]
    pub error: Value,
}
