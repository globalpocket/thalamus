use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentSpawnPayload {
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTaskAssignPayload {
    pub task_id: String,
    pub agent_id: Option<String>,
    pub parent_task_id: Option<String>,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub metadata: Value,
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub options: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTaskResultPayload {
    pub task_id: String,
    pub status: String,
    pub summary: Option<String>,
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
    pub request_id: String,
    pub task_id: Option<String>,
    pub capability: String,
    pub input: Value,
    pub agent_id: Option<String>,
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub options: Value,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeToolResultPayload {
    pub request_id: String,
    pub task_id: Option<String>,
    pub capability: String,
    pub status: String,
    pub output: Option<Value>,
    #[serde(default)]
    pub error: Value,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeLLMRequestPayload {
    pub request_id: String,
    pub task_id: Option<String>,
    pub prompt: Option<String>,
    #[serde(default)]
    pub messages: Vec<Value>,
    pub model: Option<String>,
    pub agent_id: Option<String>,
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub options: Value,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeLLMResponsePayload {
    pub request_id: String,
    pub task_id: Option<String>,
    pub status: String,
    pub text: Option<String>,
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
    pub agent_id: Option<String>,
    #[serde(default)]
    pub error: Value,
    pub task_id: Option<String>,
}
