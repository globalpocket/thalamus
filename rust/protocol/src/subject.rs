pub const RUNTIME_AGENT_READY: &str = "runtime.agent.ready";
pub const RUNTIME_AGENT_EXIT: &str = "runtime.agent.exit";
pub const RUNTIME_AGENT_ERROR: &str = "runtime.agent.error";
pub const RUNTIME_TASK_ASSIGN: &str = "runtime.task.assign";
pub const RUNTIME_TASK_ASSIGN_AGENT_TEMPLATE: &str = "runtime.task.assign.<agent_id>";

pub fn runtime_task_assign_for_agent(agent_id: &str) -> String {
    format!("runtime.task.assign.{}", agent_id)
}

pub const RUNTIME_TASK_RESULT: &str = "runtime.task.result";
pub const RUNTIME_TOOL_REQUEST: &str = "runtime.tool.request";
pub const RUNTIME_TOOL_RESULT: &str = "runtime.tool.result";
pub const RUNTIME_LLM_REQUEST: &str = "runtime.llm.request";
pub const RUNTIME_LLM_RESPONSE: &str = "runtime.llm.response";
