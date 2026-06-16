use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentSpawnPayload {
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTaskAssignPayload {
    pub task_id: String,
    pub agent_id: String,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTaskResultPayload {
    pub task_id: String,
    pub status: String,
    pub summary: Option<String>,
    pub result: Option<Value>,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeToolResultPayload {
    pub request_id: String,
    pub task_id: String,
    pub status: String,
    pub output: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuntimeLLMRequestPayload {
    pub request_id: String,
    pub task_id: Option<String>,
    pub prompt: String,
    pub model: Option<String>,
    pub agent_id: Option<String>,
}

impl<'de> Deserialize<'de> for RuntimeLLMRequestPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RuntimeLLMRequestPayloadInput {
            request_id: String,
            task_id: Option<String>,
            #[serde(default)]
            prompt: Option<String>,
            #[serde(default)]
            messages: Vec<RuntimeLLMMessageInput>,
            model: Option<String>,
            agent_id: Option<String>,
        }

        #[derive(Deserialize)]
        struct RuntimeLLMMessageInput {
            #[serde(default)]
            content: String,
        }

        let input = RuntimeLLMRequestPayloadInput::deserialize(deserializer)?;
        let last_message_content = input
            .messages
            .last()
            .map(|message| message.content.clone())
            .unwrap_or_default();
        let prompt = input
            .prompt
            .filter(|prompt| !prompt.is_empty())
            .unwrap_or(last_message_content);

        Ok(Self {
            request_id: input.request_id,
            task_id: input.task_id,
            prompt,
            model: input.model,
            agent_id: input.agent_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeLLMResponsePayload {
    pub request_id: String,
    pub task_id: String,
    pub status: String,
    pub text: Option<String>,
    pub model: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentErrorPayload {
    pub agent_id: Option<String>,
    pub error: String,
    pub task_id: Option<String>,
}
