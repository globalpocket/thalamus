use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// RuntimeState: ランタイムのライフサイクル状態
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeState {
    /// 初期状態
    Initialized,
    /// 起動中
    Starting,
    /// 実行中
    Running,
    /// 停止中
    Stopping,
    /// 停止済み
    Stopped,
}

/// TaskHandle: スパwnされたタスクのハンドル
#[derive(Clone)]
pub struct TaskHandle {
    pub id: String,
}

impl fmt::Debug for TaskHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskHandle").field("id", &self.id).finish()
    }
}

/// TaskStatus: ランタイムが追跡するタスク状態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    WaitingForLlm,
    WaitingForTool,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Assigned => "assigned",
            Self::Running => "running",
            Self::WaitingForLlm => "waiting_for_llm",
            Self::WaitingForTool => "waiting_for_tool",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn from_runtime_status(status: &str) -> Self {
        match status {
            "assigned" => Self::Assigned,
            "running" => Self::Running,
            "waiting_for_llm" => Self::WaitingForLlm,
            "waiting_for_tool" => Self::WaitingForTool,
            "completed" | "success" => Self::Completed,
            "failed" | "failure" | "error" => Self::Failed,
            "cancelled" | "canceled" => Self::Cancelled,
            _ => Self::Pending,
        }
    }
}

/// TaskState: ランタイムが追跡するタスク状態
#[derive(Debug, Clone)]
pub struct TaskState {
    pub task_id: String,
    pub parent_task_id: Arc<RwLock<Option<String>>>,
    pub agent_id: Arc<RwLock<Option<String>>>,
    pub status: Arc<RwLock<TaskStatus>>,
    pub input: Arc<RwLock<serde_json::Value>>,
    pub capabilities: Arc<RwLock<Vec<String>>>,
    pub metadata: Arc<RwLock<serde_json::Value>>,
    pub result: Arc<RwLock<Option<serde_json::Value>>>,
    pub error: Arc<RwLock<serde_json::Value>>,
    pub correlation_id: Arc<RwLock<Option<String>>>,
}

impl TaskState {
    pub fn new(id: String) -> Self {
        Self {
            task_id: id,
            parent_task_id: Arc::new(RwLock::new(None)),
            agent_id: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(TaskStatus::Pending)),
            input: Arc::new(RwLock::new(serde_json::Value::Null)),
            capabilities: Arc::new(RwLock::new(Vec::new())),
            metadata: Arc::new(RwLock::new(serde_json::json!({}))),
            result: Arc::new(RwLock::new(None)),
            error: Arc::new(RwLock::new(serde_json::Value::Null)),
            correlation_id: Arc::new(RwLock::new(None)),
        }
    }

    pub fn id(&self) -> &str {
        &self.task_id
    }

    pub async fn assign_to(&self, agent_id: String) {
        *self.agent_id.write().await = Some(agent_id);
        self.set_task_status(TaskStatus::Assigned).await;
    }

    pub async fn assigned_agent(&self) -> Option<String> {
        self.agent_id.read().await.clone()
    }

    pub async fn set_status(&self, status: String) {
        self.set_task_status(TaskStatus::from_runtime_status(&status))
            .await;
    }

    pub async fn set_task_status(&self, status: TaskStatus) {
        *self.status.write().await = status;
    }

    pub async fn status(&self) -> String {
        self.status.read().await.as_str().to_string()
    }
}
